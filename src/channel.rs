// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{
    boxed::Box,
    collections::VecDeque,
    fmt::Debug,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use erpc_sys::{
    c_int, c_void,
    erpc::{SmErrType, SmEventType},
};

use crate::{
    buf::MsgBufferReader, call::RpcCall, env::Environment, error::Result, msg_buffer::MsgBuffer,
    nexus::Nexus, rpc::Rpc, server::ServerRpcContext,
};

pub enum RpcContext {
    Client(ClientRpcContext),
    Server(ServerRpcContext),
}

pub struct ClientRpcContext {
    // TODO: make sure resp of the same type are not out of order, otherwise use `HashMap` instead
    pub resp_msgbufs: Vec<VecDeque<Arc<MsgBuffer>>>,
}

pub type RpcPollFn = Box<dyn Fn(u8, &mut Arc<Nexus>, Sender<Channel>) + Send + 'static>;

pub struct ChannelBuilder {
    env: Arc<Environment>,
    subchan_count: usize,
    phy_port: u8,
    timeout_ms: usize,
}

// TODO: impl this to make sure session has been connected before use
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

    pub async fn connect<S: Into<String>>(self, uri: S) -> Result<Channel> {
        let env = self.env.pick_channel_env().unwrap();
        let uri = uri.into();
        env.0
            .send(Box::new(
                move |id: u8, nexus: &mut Arc<Nexus>, chan_tx: Sender<Channel>| {
                    let mut ctx = ClientRpcContext {
                        // TODO: make cap configurable, related with eRPC's credits
                        resp_msgbufs: vec![VecDeque::with_capacity(32); MAX_REQ_TYPE],
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
                    let rpc_clone = rpc.clone();
                    let rpc = unsafe { Arc::get_mut_unchecked(&mut rpc) };
                    let mut subchans = Vec::new();
                    for _i in 0..self.subchan_count {
                        // TODO: make rem_rpc_id configurable
                        subchans.push(rpc.create_session(uri.as_str(), 0).unwrap());
                    }
                    let chan = Channel {
                        subchans,
                        assigned_idx: Arc::new(AtomicUsize::new(0)),
                        rpc: rpc_clone,
                        tx,
                    };
                    chan_tx.send_blocking(chan).unwrap();

                    loop {
                        match rx.try_recv() {
                            Ok(call) => call.resolve(rpc, raw_ctx),
                            Err(TryRecvError::Empty) => {}
                            Err(TryRecvError::Closed) => {}
                        }
                        rpc.run_event_loop(self.timeout_ms);
                    }
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
            let (tx, rx) = bounded::<MsgBufferReader>(1);
            return Some(SubChannel {
                id: self.subchans[idx],
                rpc: self.rpc.clone(),
                tx: self.tx.clone(),
                mbr_tx: Box::into_raw(Box::new(tx)),
                mbr_rx: rx,
            });
        }
        None
    }
}

#[derive(Clone)]
pub struct SubChannel {
    pub id: c_int,
    pub rpc: Arc<Rpc>,
    pub tx: Sender<RpcCall>,
    pub mbr_tx: *mut Sender<MsgBufferReader>,
    pub mbr_rx: Receiver<MsgBufferReader>,
}
