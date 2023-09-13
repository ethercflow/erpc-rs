use clap::Parser;

const K_APP_MAX_CONCURRENCY: usize = 32;

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
    /// NUMA node for this process
    #[arg(short, long)]
    pub numa_node: usize,
    /// Fabric ports on NUMA node 0, CSV, no spaces
    #[arg(short, long)]
    pub numa_0_ports: String,
    /// Fabric ports on NUMA node 1, CSV, no spaces
    #[arg(short, long)]
    pub numa_1_ports: String,
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

impl Args {
    pub fn flags_get_num_ports(&self) -> Vec<u8> {
        if self.numa_node > 1 {
            panic!("Only NUMA 0 and NUMA 1 supported");
        }
        let ports = if self.numa_node == 0 {
            &self.numa_0_ports
        } else {
            &self.numa_1_ports
        };
        ports
            .split(',')
            .map(|p| p.parse::<u8>().unwrap())
            .collect::<Vec<u8>>()
    }
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
