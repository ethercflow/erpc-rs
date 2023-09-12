// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::cli::Args;
use crate::context::AppContext;
use erpc_rs::prelude::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub fn connect_sessions_func_victim(
    c: &mut AppContext,
    args: Arc<Args>,
    ctrl_c_pressed: Arc<AtomicBool>,
) {
    if args.process_id == 0 || args.process_id == args.num_processes - 1 {
        return;
    }

    let global_thread_id = args.process_id * args.num_proc_other_threads + c.base.thread_id;

    let mut server_process_id = 0_usize;
    let mut rem_tid = 0_u8;
    if args.process_id != args.num_processes - 2 {
        rem_tid = (global_thread_id % args.num_proc_0_threads)
            .try_into()
            .unwrap();
    } else if c.base.thread_id == args.num_proc_other_threads - 1 {
        server_process_id = args.num_processes - 1;
    } else {
        rem_tid = (global_thread_id % args.num_proc_0_threads)
            .try_into()
            .unwrap();
    }

    c.base.session_num_vec.resize(1, c_int(0));

    println!(
        "large_rpc_tput: Thread {}: Creating session to proc {}, thread {}.",
        c.base.thread_id, server_process_id, rem_tid
    );

    c.base.session_num_vec[0] = unsafe { c.base.rpc.assume_init_mut() }.create_session(
        (*get_uri_for_process(server_process_id)).to_str().unwrap(),
        rem_tid,
    );
    if i32::from(c.base.session_num_vec[0]) < 0 {
        panic!("create_session failed.");
    }

    loop {
        if c.base.num_sm_resps == 1 {
            break;
        }
        unsafe { c.base.rpc.assume_init_mut() }.run_event_loop(200);
        if ctrl_c_pressed.load(Ordering::Relaxed) {
            return;
        }
    }

    // If throttling is enabled, flows to the incast victim are throttled
    if server_process_id == 0 && args.throttle == 1.0 {
        let mut timely_0 =
            unsafe { c.base.rpc.assume_init_mut() }.get_timely(c.base.session_num_vec[0]);
        let num_incast_flows = (args.num_processes - 2) * args.num_proc_other_threads - 1;
        let fair_share = unsafe { c.base.rpc.assume_init_mut() }.get_bandwidth() / num_incast_flows;
        timely_0.set_rate(fair_share as f64 * args.throttle_fraction);
    }
}
