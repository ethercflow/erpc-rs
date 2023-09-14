// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::K_APP_DATA_BYTE;
use autocxx::WithinUniquePtr;
use erpc_rs::prelude::*;
use std::{mem::MaybeUninit, ops::Add, ptr};

const K_APP_MAX_CONCURRENCY: usize = 32;

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct AppStats {
    pub rx_gbps: f64,
    pub tx_gbps: f64,
    pub re_tx: usize,
    // Median packet RTT
    pub rtt_50_us: f64,
    // 99th percentile packet RTT
    pub rtt_99_us: f64,
    pub rpc_50_us: f64,
    pub rpc_99_us: f64,
    pub rpc_999_us: f64,
}

impl Add for AppStats {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            rx_gbps: self.rx_gbps + rhs.rx_gbps,
            tx_gbps: self.tx_gbps + rhs.tx_gbps,
            re_tx: self.re_tx + rhs.re_tx,
            rtt_50_us: self.rtt_50_us + rhs.rtt_50_us,
            rtt_99_us: self.rtt_99_us + rhs.rtt_99_us,
            rpc_50_us: self.rpc_50_us + rhs.rpc_50_us,
            rpc_99_us: self.rpc_99_us + rhs.rpc_99_us,
            rpc_999_us: self.rpc_999_us + rhs.rpc_999_us,
        }
    }
}

/// Base class for per-thread application context
pub struct BasicAppContext {
    pub rpc: MaybeUninit<Rpc>,
    pub fastrand: UniquePtr<FastRand>,

    pub session_num_vec: Vec<c_int>,

    /// The ID of the thread that owns this context
    pub thread_id: usize,
    /// Number of SM responses
    pub num_sm_resps: usize,
    /// Only one ping is allowed at a time
    pub ping_pending: bool,
}

impl Default for BasicAppContext {
    fn default() -> Self {
        BasicAppContext {
            rpc: MaybeUninit::uninit(),
            fastrand: FastRand::new().within_unique_ptr(),
            session_num_vec: Vec::default(),
            thread_id: usize::MAX,
            num_sm_resps: 0,
            ping_pending: false,
        }
    }
}

impl BasicAppContext {
    #[allow(dead_code)]
    pub fn fast_get_rand_session_num(&mut self) -> c_int {
        let x = self.fastrand.pin_mut().next_u32();
        let rand_index = (x as usize * self.session_num_vec.len()) >> 32;
        self.session_num_vec[rand_index]
    }
}

impl Drop for BasicAppContext {
    fn drop(&mut self) {
        unsafe {
            self.rpc.assume_init_drop();
        }
    }
}

pub struct AppContext {
    pub base: BasicAppContext,

    // We need a wide range of latency measurements: ~4 us for 4KB RPCs, to
    // >10 ms for 8MB RPCs under congestion. So erpc::Latency doesn't work here.
    pub lat_vec: Vec<f64>,

    pub tput_t0: UniquePtr<ChronoTimer>,
    pub app_stats: AppStats,

    pub stat_rx_bytes_tot: usize,
    pub stat_tx_bytes_tot: usize,
    pub req_ts: [usize; K_APP_MAX_CONCURRENCY],
    pub req_msgbuf: [MaybeUninit<MsgBuffer>; K_APP_MAX_CONCURRENCY],
    pub resp_msgbuf: [MaybeUninit<MsgBuffer>; K_APP_MAX_CONCURRENCY],
    pub msgbuf_nr: usize,

    pub args_req_size: usize,
    pub args_resp_size: usize,
    pub args_process_id: usize,
    pub args_sm_verbose: bool,
}

impl Default for AppContext {
    fn default() -> Self {
        AppContext {
            base: BasicAppContext::default(),
            lat_vec: Vec::default(),
            tput_t0: ChronoTimer::new().within_unique_ptr(),
            app_stats: AppStats::default(),
            stat_rx_bytes_tot: 0,
            stat_tx_bytes_tot: 0,
            req_ts: [0; K_APP_MAX_CONCURRENCY],
            req_msgbuf: MaybeUninit::uninit_array(),
            resp_msgbuf: MaybeUninit::uninit_array(),
            msgbuf_nr: 0,
            args_req_size: 0,
            args_resp_size: 0,
            args_process_id: 0,
            args_sm_verbose: false,
        }
    }
}

impl AppContext {
    pub fn alloc_req_resp_msg_buffers(&mut self, num: usize) {
        for i in 0..num {
            unsafe {
                self.req_msgbuf[i].as_mut_ptr().write(
                    self.base
                        .rpc
                        .assume_init_mut()
                        .alloc_msg_buffer_or_die(self.args_req_size),
                );
                ptr::write_bytes(
                    self.req_msgbuf[i].assume_init_mut().get_inner_buf(),
                    K_APP_DATA_BYTE,
                    self.args_req_size,
                );
            }
            unsafe {
                self.resp_msgbuf[i].as_mut_ptr().write(
                    self.base
                        .rpc
                        .assume_init_mut()
                        .alloc_msg_buffer_or_die(self.args_resp_size),
                );
            }
        }
        self.msgbuf_nr = num;
    }
}

impl Drop for AppContext {
    fn drop(&mut self) {
        for i in 0..self.msgbuf_nr {
            unsafe {
                self.req_msgbuf[i].assume_init_drop();
                self.resp_msgbuf[i].assume_init_drop();
            }
        }
    }
}
