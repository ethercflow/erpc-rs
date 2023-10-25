// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use clap::Parser;

use crate::common::K_APP_MAX_CONCURRENCY;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Test milliseconds
    #[arg(short, long)]
    pub test_ms: u64,
    /// Print session management debug info
    #[arg(short, long)]
    pub sm_verbose: bool,
    /// Number of eRPC processes in the cluster
    #[arg(short, long)]
    pub num_processes: usize,
    /// The global ID of this process
    #[arg(short, long, default_value_t = usize::MAX)]
    pub process_id: usize,
    /// The phy port of the nic
    #[arg(short, long)]
    pub phy_port: u8,
    /// Threads in process 0
    #[arg(short, long)]
    pub num_proc_0_threads: usize,
    /// "Threads in process with ID != 0"
    #[arg(short, long)]
    pub num_proc_other_threads: usize,
    /// "Request data size"
    #[arg(short, long)]
    pub req_size: usize,
    /// Response data size
    #[arg(short, long)]
    pub resp_size: usize,
    /// Concurrent requests per thread
    #[arg(short, long)]
    pub concurrency: usize,
    /// Packet drop probability
    #[arg(short, long)]
    pub drop_prob: f64,
    /// Experiment profile to use
    #[arg(short, long)]
    pub profile: String,
    /// Throttle flows to incast receiver?
    #[arg(short, long)]
    pub throttle: f64,
    /// Fraction of fair share to throttle to.
    #[arg(short, long, default_value_t = 1.0)]
    pub throttle_fraction: f64,
}

pub fn parse_args() -> Args {
    let args = Args::parse();
    if args.concurrency > K_APP_MAX_CONCURRENCY {
        panic!("Invalid conc");
    }
    if args.profile != "incast" && args.profile != "victim" {
        panic!("Invalid profile");
    }
    if args.process_id >= args.num_processes {
        panic!("Invalid process ID");
    }
    if args.drop_prob >= 1.0 {
        panic!("Invalid drop prob");
    }
    args
}
