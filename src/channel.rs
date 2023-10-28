// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{
    boxed::Box,
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use erpc_sys::{
    c_int, c_void,
    erpc::{ms_to_cycles, rdtsc, SmErrType, SmEventType},
};

use crate::{
    call::RpcCall, env::Environment, error::Result, msg_buffer::MsgBuffer, nexus::Nexus, rpc::Rpc,
};

#[cfg(feature = "bench_stat")]
use crate::stat::BenchStat;

pub struct ClientRpcContext {
    pub resp_msgbufs: Vec<HashMap<u16, Arc<MsgBuffer>>>,
    pub resp_msgbufs_idxs: Vec<u16>,

    pub rpc: Option<Arc<Rpc>>,

    #[cfg(feature = "bench_stat")]
    pub bench_stat: BenchStat,
}

impl Default for ClientRpcContext {
    fn default() -> Self {
        ClientRpcContext {
            resp_msgbufs: vec![HashMap::new(); MAX_REQ_TYPE],
            resp_msgbufs_idxs: vec![0; MAX_REQ_TYPE],
            rpc: None,
            #[cfg(feature = "bench_stat")]
            bench_stat: Default::default(),
        }
    }
}

pub type RpcPollFn = Box<dyn Fn(u8, &mut Arc<Nexus>, Sender<Channel>) + Send + 'static>;

pub struct ChannelBuilder {
    env: Arc<Environment>,
    subchan_count: usize,
    phy_port: u8,
    timeout_ms: usize,
    #[cfg(feature = "bench_stat")]
    req_size: usize,
    #[cfg(feature = "bench_stat")]
    resp_size: usize,
}

// TODO: impl this to make sure session has been connected before us
extern "C" fn sm_handler(_: c_int, _: SmEventType, _: SmErrType, _: *mut c_void) {}

const MAX_REQ_TYPE: usize = 257;

impl ChannelBuilder {
    /// Initialize a new [`ChannelBuilder`].
    pub fn new(env: Arc<Environment>, port: u8) -> Self {
        ChannelBuilder {
            env,
            subchan_count: 128,
            phy_port: port,
            timeout_ms: 0,
            #[cfg(feature = "bench_stat")]
            req_size: 0,
            #[cfg(feature = "bench_stat")]
            resp_size: 0,
        }
    }

    /// This method will panic if `count` is 0.
    pub fn subchan_count(mut self, count: usize) -> ChannelBuilder {
        assert!(count > 0);
        self.subchan_count = count;
        self
    }

    /// Set timeout ms.
    pub fn timeout_ms(mut self, timeout_ms: usize) -> ChannelBuilder {
        self.timeout_ms = timeout_ms;
        self
    }

    #[cfg(feature = "bench_stat")]
    /// Set req_size
    pub fn req_size(mut self, req_size: usize) -> ChannelBuilder {
        self.req_size = req_size;
        self
    }

    #[cfg(feature = "bench_stat")]
    /// Set resp_size
    pub fn resp_size(mut self, resp_size: usize) -> ChannelBuilder {
        self.resp_size = resp_size;
        self
    }

    pub async fn connect<S: Into<String>>(self, uri: S) -> Result<Channel> {
        let env = self.env.pick_channel_env().unwrap();
        let uri = uri.into();
        env.0
            .send(Box::new(
                move |id: u8, nexus: &mut Arc<Nexus>, chan_tx: Sender<Channel>| {
                    #[cfg(feature = "bench_stat")]
                    let bench_stat = BenchStat {
                        thread_id: id as usize,
                        args_req_size: self.req_size,
                        args_resp_size: self.resp_size,
                        ..Default::default()
                    };
                    let mut ctx = ClientRpcContext {
                        #[cfg(feature = "bench_stat")]
                        bench_stat,
                        ..Default::default()
                    };
                    let raw_ctx = &mut ctx as *mut ClientRpcContext as *mut c_void;
                    let (tx, rx) = unbounded::<RpcCall>();
                    let mut rpc = Arc::new(Rpc::new(
                        unsafe { Arc::get_mut_unchecked(nexus) },
                        Some(raw_ctx),
                        id,
                        Some(sm_handler),
                        self.phy_port,
                    ));
                    ctx.rpc = Some(rpc.clone());
                    let rpc_clone = rpc.clone();
                    let rpc = unsafe { Arc::get_mut_unchecked(&mut rpc) };
                    let mut subchans = Vec::new();
                    for _i in 0..self.subchan_count {
                        // TODO: make rem_rpc_id configurable
                        let sid = rpc.create_session(uri.as_str(), 0).unwrap();
                        loop {
                            rpc.run_event_loop_once();
                            if rpc.is_connected(sid) {
                                break;
                            }
                        }
                        subchans.push(sid);
                    }
                    let (stx, srx) = bounded::<()>(1);
                    let chan = Channel {
                        subchans: subchans.clone(),
                        assigned_idx: Arc::new(AtomicUsize::new(0)),
                        rpc: rpc_clone,
                        tx,
                        rx: srx,
                    };
                    chan_tx.send_blocking(chan).unwrap();

                    #[cfg(feature = "bench_stat")]
                    ctx.bench_stat.init();

                    'outer: loop {
                        let timeout_tsc = ms_to_cycles(self.timeout_ms as f64, rpc.get_freq_ghz());
                        let start_tsc = rdtsc();
                        loop {
                            rpc.run_event_loop_once();
                            // TODO: make it configurable
                            for _i in 0..8192 {
                                match rx.try_recv() {
                                    Ok(call) => call.resolve(rpc, raw_ctx),
                                    Err(TryRecvError::Empty) => break,
                                    Err(TryRecvError::Closed) => {
                                        break 'outer;
                                    }
                                }
                            }
                            if rpc.get_ev_loop_tsc() - start_tsc > timeout_tsc {
                                break;
                            }
                        }

                        #[cfg(feature = "bench_stat")]
                        {
                            let mut timely = rpc.get_timely(c_int::from(0));

                            ctx.bench_stat.compute(
                                rpc.get_num_re_tx(subchans[0]),
                                self.timeout_ms,
                                timely.get_rtt_perc(0.5),
                                timely.get_rtt_perc(0.99),
                            );
                            ctx.bench_stat.output(timely.get_rate_gbps());
                            ctx.bench_stat.reset();
                            timely.reset_rtt_stats();
                            rpc.reset_num_re_tx(subchans[0]);
                        }
                    }

                    for sid in subchans {
                        rpc.destroy_session(sid).unwrap();
                    }
                    stx.send_blocking(()).unwrap();
                },
            ))
            .await
            .unwrap();
        env.1.recv().await.map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct Channel {
    pub subchans: Vec<c_int>,
    pub assigned_idx: Arc<AtomicUsize>,
    pub rpc: Arc<Rpc>,
    pub tx: Sender<RpcCall>,
    pub rx: Receiver<()>,
}

impl Debug for Channel {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

impl Channel {
    pub fn pick_subchan(&mut self) -> Option<SubChannel> {
        let idx = self.assigned_idx.fetch_add(1, Ordering::Relaxed);
        if idx < self.subchans.len() {
            return Some(SubChannel {
                id: self.subchans[idx],
                rpc: self.rpc.clone(),
                tx: self.tx.clone(),
            });
        }
        None
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.tx.close();
        self.rx.recv().await.map_err(Into::into)
    }
}

#[derive(Clone)]
pub struct SubChannel {
    pub id: c_int,
    pub rpc: Arc<Rpc>,
    pub tx: Sender<RpcCall>,
}
