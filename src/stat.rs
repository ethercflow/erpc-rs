// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use erpc_sys::{
    erpc::{kSessionCredits, ChronoTimer},
    UniquePtr, WithinUniquePtr,
};

pub const MAX_CONCURRENCY: usize = 32;

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct TransStats {
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

pub struct BenchStat {
    /// The ID of the thread that owns this context
    pub thread_id: usize,
    // We need a wide range of latency measurements: ~4 us for 4KB RPCs, to
    // >10 ms for 8MB RPCs under congestion. So erpc::Latency doesn't work here.
    pub lat_vec: Vec<f64>,

    pub tput: UniquePtr<ChronoTimer>,
    pub tr_stats: TransStats,

    pub stat_rx_bytes_tot: usize,
    pub stat_tx_bytes_tot: usize,
    pub req_ts: [usize; MAX_CONCURRENCY],

    pub args_req_size: usize,
    pub args_resp_size: usize,
}

impl Default for BenchStat {
    fn default() -> Self {
        BenchStat {
            thread_id: usize::MAX,
            lat_vec: Vec::default(),
            tput: ChronoTimer::new().within_unique_ptr(),
            tr_stats: TransStats::default(),
            stat_rx_bytes_tot: 0,
            stat_tx_bytes_tot: 0,
            req_ts: [0; MAX_CONCURRENCY],
            args_req_size: 0,
            args_resp_size: 0,
        }
    }
}

impl BenchStat {
    pub fn init(&mut self) {
        self.tput.pin_mut().as_mut().reset();
    }

    pub fn compute(&mut self, re_tx: usize, timeout_ms: usize, rtt_50_us: f64, rtt_99_us: f64) {
        let ns = self.tput.pin_mut().get_ns();
        // Publish stats
        self.tr_stats.rx_gbps = self.stat_rx_bytes_tot as f64 * 8f64 / ns as f64;
        self.tr_stats.tx_gbps = self.stat_tx_bytes_tot as f64 * 8f64 / ns as f64;
        self.tr_stats.re_tx = re_tx;
        self.tr_stats.rtt_50_us = rtt_50_us;
        self.tr_stats.rtt_99_us = rtt_99_us;

        if !self.lat_vec.is_empty() {
            self.lat_vec.sort_floats();
            self.tr_stats.rpc_50_us =
                self.lat_vec[(self.lat_vec.len() as f64 * 0.5).floor() as usize];
            self.tr_stats.rpc_99_us =
                self.lat_vec[(self.lat_vec.len() as f64 * 0.99).floor() as usize];
            self.tr_stats.rpc_999_us =
                self.lat_vec[(self.lat_vec.len() as f64 * 0.999).floor() as usize];
        } else {
            // Even if no RPCs completed, we need retransmission counter
            self.tr_stats.rpc_50_us = timeout_ms as f64 * 1000.0;
            self.tr_stats.rpc_99_us = timeout_ms as f64 * 1000.0;
            self.tr_stats.rpc_999_us = timeout_ms as f64 * 1000.0;
        }
    }

    pub fn output(&self, rate_gbps: f64) {
        let stats = format!(
                            "Rpc throughput: Thread {}: Tput {{RX {:.2} ({}), TX {:.2} ({})}} \
                             Gbps (IOPS). Retransmissions {}. Packet RTTs: {{{:.1}, {:.1}}} us. \
                             RPC latency {{{:.1} 50th, {:.1} 99th, {:.1} 99.9th}}. Timely rate {:.1} \
                             Gbps. Credits {} (best = 32).",
                            self.thread_id,
                            self.tr_stats.rx_gbps,
                            self.stat_rx_bytes_tot / self.args_resp_size,
                            self.tr_stats.tx_gbps,
                            self.stat_tx_bytes_tot / self.args_req_size,
                            self.tr_stats.re_tx,
                            self.tr_stats.rtt_50_us,
                            self.tr_stats.rtt_99_us,
                            self.tr_stats.rpc_50_us,
                            self.tr_stats.rpc_99_us,
                            self.tr_stats.rpc_999_us,
                            rate_gbps,
                            kSessionCredits
                        );
        println!("{}", stats);
    }

    pub fn reset(&mut self) {
        // Reset stats for next iteration
        self.stat_rx_bytes_tot = 0;
        self.stat_tx_bytes_tot = 0;
        self.lat_vec.clear();
        self.tput.pin_mut().as_mut().reset();
    }
}
