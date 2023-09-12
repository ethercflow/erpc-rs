// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(sort_floats)]

mod cli;
mod context;
mod profile_incast;
mod profile_victim;

use cli::*;
use context::*;
use erpc_rs::prelude::*;
use profile_incast::*;
use profile_victim::*;
use signal_hook::consts::SIGINT;
use std::{
    io::Error,
    mem::{self, MaybeUninit},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

const K_APP_REQ_TYPE: u8 = 1;
const K_APP_DATA_BYTE: u8 = 3;
const K_APP_EV_LOOP_MS: usize = 1000;

type ConnectSessionsFunc = fn(&mut AppContext, args: Arc<Args>, Arc<AtomicBool>);

// Profile-specifc session connection function
static mut CONNECT_SESSIONS_FUNC: MaybeUninit<ConnectSessionsFunc> = MaybeUninit::uninit();

// Use the supplied profile set up globals and possibly modify other flags
fn setup_profile(args: &Args) {
    match args.profile.as_str() {
        "incast" => unsafe {
            CONNECT_SESSIONS_FUNC
                .as_mut_ptr()
                .write(connect_sessions_func_incast);
        },
        "victim" => {
            if args.num_processes < 3 {
                panic!("Too few processes");
            }
            if args.num_proc_other_threads < 2 {
                panic!("Too few threads");
            }
            unsafe {
                CONNECT_SESSIONS_FUNC
                    .as_mut_ptr()
                    .write(connect_sessions_func_victim);
            }
        }
        _ => unreachable!(),
    }
}

extern "C" fn basic_sm_handler(
    session_num: c_int,
    sm_event_type: SmEventType,
    sm_err_type: SmErrType,
    context: *mut c_void,
) {
    let c = unsafe { &mut *(context as *mut AppContext) };
    c.base.num_sm_resps += 1;

    if sm_err_type != SmErrType::kNoError {
        panic!("SM response with error {}", sm_err_type_str(sm_err_type));
    }
    if sm_event_type != SmEventType::kConnected && sm_event_type != SmEventType::kDisconnected {
        panic!("Received unexpected SM event.");
    }

    // The callback gives us the eRPC session number - get the index in vector
    let mut session_idx = c.base.session_num_vec.len();
    for i in 0..c.base.session_num_vec.len() {
        if c.base.session_num_vec[i] == session_num {
            session_idx = i;
            break;
        }
    }
    if session_idx >= c.base.session_num_vec.len() {
        panic!("SM callback for invalid session number.");
    }

    if c.args_sm_verbose {
        let verbose = format!(
            "Process {}, Rpc {}: Session number {:?} (index {}) {}. Error {}. \
             Time elapsed = {:.3} s.",
            c.args_process_id,
            unsafe { c.base.rpc.assume_init_mut() }.get_rpc_id(),
            session_num,
            session_idx,
            sm_event_type_str(sm_event_type),
            sm_err_type_str(sm_err_type),
            unsafe { c.base.rpc.assume_init_mut() }.sec_since_creation()
        );
        eprintln!("{}", verbose);
    }
}

extern "C" fn app_cont_func(context: *mut c_void, tag: *mut c_void) {
    let c = unsafe { &mut *(context as *mut AppContext) };
    let msgbuf_idx = tag as usize;

    let resp_msgbuf = unsafe { &c.resp_msgbuf.assume_init_mut()[msgbuf_idx] };

    // Measure latency. 1 us granularity is sufficient for large RPC latency.
    let usec = to_usec(
        rdtsc() - c.req_ts[msgbuf_idx],
        unsafe { c.base.rpc.assume_init_mut() }.get_freq_ghz(),
    );
    c.lat_vec.push(usec);

    // Check the response
    if resp_msgbuf.get_data_size() != c.args_resp_size {
        panic!("Invalid response size");
    }
    if unsafe { *resp_msgbuf.get_inner_buf().wrapping_offset(0) } != K_APP_DATA_BYTE {
        panic!("Invalid resp data");
    }

    c.stat_rx_bytes_tot += c.args_resp_size;

    // Create a new request clocking this response, and put in request queue
    unsafe {
        *c.req_msgbuf.assume_init_mut()[msgbuf_idx]
            .get_inner_buf()
            .wrapping_offset(0) = K_APP_DATA_BYTE;
    }

    send_req(c, msgbuf_idx);
}

extern "C" fn req_handler(req_handle: *mut RawReqHandle, context: *mut c_void) {
    let c = unsafe { &mut *(context as *mut AppContext) };
    let mut req_handle = ReqHandle::from_inner_raw(req_handle);
    let req_msgbuf = req_handle.get_req_msgbuf();
    let resp_byte = unsafe { *(*req_msgbuf).get_inner_buf().wrapping_offset(0) };

    // Use dynamic response
    let mut resp_msgbuf = req_handle.get_dyn_resp_msgbuf();
    unsafe {
        mem::swap(
            &mut resp_msgbuf,
            &mut c
                .base
                .rpc
                .assume_init_mut()
                .alloc_msg_buffer_or_die(c.args_resp_size),
        );

        // Touch the response
        *resp_msgbuf.get_inner_buf().wrapping_offset(0) = resp_byte;
    }

    c.stat_rx_bytes_tot += c.args_req_size;
    c.stat_tx_bytes_tot += c.args_resp_size;

    unsafe {
        c.base
            .rpc
            .assume_init_mut()
            .enqueue_response(&mut req_handle, &mut resp_msgbuf);
    }
}

// Send a request using this MsgBuffer
fn send_req(c: &mut AppContext, idx: usize) {
    let req_msgbuf = unsafe { &mut c.req_msgbuf.assume_init_mut()[idx] };
    let resp_msgbuf = unsafe { &mut c.resp_msgbuf.assume_init_mut()[idx] };
    if req_msgbuf.get_data_size() != c.args_req_size {
        panic!("allocated req_msgbuf's data size not eq args' req_size");
    }

    c.req_ts[idx] = rdtsc();
    unsafe {
        c.base.rpc.assume_init_mut().enqueue_request(
            c.base.session_num_vec[0],
            K_APP_REQ_TYPE,
            req_msgbuf,
            resp_msgbuf,
            app_cont_func,
            Some(idx as *mut c_void),
        );
    }

    c.stat_tx_bytes_tot += c.args_req_size;
}

fn thread_func(
    thread_id: usize,
    args: Arc<Args>,
    nexus: Arc<Mutex<Nexus>>,
    ctrl_c_pressed: Arc<AtomicBool>,
) {
    let mut c = AppContext::default();
    c.base.thread_id = thread_id;
    c.args_req_size = args.req_size;
    c.args_resp_size = args.resp_size;
    c.args_process_id = args.process_id;
    c.args_sm_verbose = args.sm_verbose;

    let ports = args.flags_get_num_ports();
    if ports.is_empty() {
        panic!("no available port");
    }
    let phy_port = *ports.get(thread_id % ports.len()).unwrap();

    let mut rpc = {
        let mut nexus = nexus.lock().unwrap();
        Rpc::new(
            &mut nexus,
            Some(&mut c as *mut AppContext as *mut c_void),
            thread_id as u8,
            Some(basic_sm_handler),
            phy_port,
        )
    };
    rpc.force_retry_connect_on_invalid_rpc_id();

    unsafe {
        c.base.rpc.as_mut_ptr().write(rpc);
    }

    unsafe {
        // Create the session. Some threads may not create any sessions, and therefore
        // not run the event loop required for other threads to connect them. This
        // is OK because all threads will run the event loop below.
        CONNECT_SESSIONS_FUNC.assume_init()(&mut c, args.clone(), ctrl_c_pressed.clone());
    }
    if !c.base.session_num_vec.is_empty() {
        println!(
            "large_rpc_tput: Thread {}: All sessions connected.\n",
            thread_id
        );
    } else {
        println!(
            "large_rpc_tput: Thread {}: No sessions created.\n",
            thread_id,
        );
    }

    // All threads allocate MsgBuffers, but they may not send requests
    c.alloc_req_resp_msg_buffers(args.concurrency);

    let console_ref_tsc = rdtsc();

    // Any thread that creates a session sends requests
    if !c.base.session_num_vec.is_empty() {
        for i in 0..args.concurrency {
            send_req(&mut c, i);
        }
    }

    let mut tput_t0 = c.tput_t0.pin_mut();
    tput_t0.as_mut().reset();
    for _i in (0..args.test_ms).step_by(K_APP_EV_LOOP_MS) {
        unsafe {
            c.base
                .rpc
                .assume_init_mut()
                .run_event_loop(K_APP_EV_LOOP_MS);
        }
        if ctrl_c_pressed.load(Ordering::Relaxed) {
            break;
        }
        // No stats to print
        if c.base.session_num_vec.is_empty() {
            continue;
        }

        let ns = tput_t0.get_ns();
        let mut timely_0 = unsafe { c.base.rpc.assume_init_mut().get_timely(c_int::from(0)) };

        // Publish stats
        c.app_stats.rx_gbps = c.stat_rx_bytes_tot as f64 * 8f64 / ns as f64;
        c.app_stats.tx_gbps = c.stat_tx_bytes_tot as f64 * 8f64 / ns as f64;
        c.app_stats.re_tx = unsafe {
            c.base
                .rpc
                .assume_init_mut()
                .get_num_re_tx(c.base.session_num_vec[0])
        };
        c.app_stats.rtt_50_us = timely_0.get_rtt_perc(0.5);
        c.app_stats.rtt_99_us = timely_0.get_rtt_perc(0.99);

        if !c.lat_vec.is_empty() {
            c.lat_vec.sort_floats();
            c.app_stats.rpc_50_us = c.lat_vec[(c.lat_vec.len() as f64 * 0.5).floor() as usize];
            c.app_stats.rpc_99_us = c.lat_vec[(c.lat_vec.len() as f64 * 0.99).floor() as usize];
            c.app_stats.rpc_999_us = c.lat_vec[(c.lat_vec.len() as f64 * 0.999).floor() as usize];
        } else {
            // Even if no RPCs completed, we need retransmission counter
            c.app_stats.rpc_50_us = K_APP_EV_LOOP_MS as f64 * 1000.0;
            c.app_stats.rpc_99_us = K_APP_EV_LOOP_MS as f64 * 1000.0;
            c.app_stats.rpc_999_us = K_APP_EV_LOOP_MS as f64 * 1000.0;
        }

        let stats = format!(
            "large_rpc_tput: Thread {}: Tput {{RX {:.2} ({}), TX {:.2} ({})}} \
             Gbps (IOPS). Retransmissions {}. Packet RTTs: {{{:.1}, {:.1}}} us. \
             RPC latency {{{:.1} 50th, {:.1} 99th, {:.1} 99.9th}}. Timely rate {:.1} \
             Gbps. Credits {} (best = 32).",
            c.base.thread_id,
            c.app_stats.rx_gbps,
            c.stat_rx_bytes_tot / c.args_resp_size,
            c.app_stats.tx_gbps,
            c.stat_tx_bytes_tot / c.args_req_size,
            c.app_stats.re_tx,
            c.app_stats.rtt_50_us,
            c.app_stats.rtt_99_us,
            c.app_stats.rpc_50_us,
            c.app_stats.rpc_99_us,
            c.app_stats.rpc_999_us,
            timely_0.get_rate_gbps(),
            kSessionCredits
        );
        println!("{}", stats);

        // Reset stats for next iteration
        c.stat_rx_bytes_tot = 0;
        c.stat_tx_bytes_tot = 0;
        unsafe {
            c.base
                .rpc
                .assume_init_mut()
                .reset_num_re_tx(c.base.session_num_vec[0]);
        }
        c.lat_vec.clear();
        timely_0.reset_rtt_stats();
        tput_t0.as_mut().reset();
    }

    let mut wheel = unsafe { c.base.rpc.assume_init_mut().get_wheel() };
    if !wheel.is_none() && !wheel.get_record_vec().is_empty() {
        let num_to_print = 200_usize;
        let tot_entries = wheel.get_record_vec().len();
        let base_entry = (tot_entries as f64 * 0.9).floor() as usize;

        println!("Printing up to 200 entries toward the end of wheel record");
        let mut num_printed = 0_usize;

        let mut records = wheel.get_record_vec();
        for i in base_entry..tot_entries {
            let record = records.as_mut().index_mut(i).unwrap();
            println!(
                "wheel: {}",
                record
                    .to_string(console_ref_tsc, unsafe {
                        c.base.rpc.assume_init_mut().get_freq_ghz()
                    })
                    .to_str()
                    .unwrap()
            );
            num_printed += 1;
            if num_printed == num_to_print {
                break;
            }
        }
    }
}

fn main() -> Result<(), Error> {
    let ctrl_c_pressed: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&ctrl_c_pressed))?;
    let args = parse_args();

    setup_profile(&args);

    let mut nexus = Nexus::new(
        (*get_uri_for_process(args.process_id)).to_str().unwrap(),
        args.numa_node,
    );
    nexus
        .register_req_func(K_APP_REQ_TYPE, req_handler)
        .unwrap();

    let num_threads = if args.process_id == 0 {
        args.num_proc_0_threads
    } else {
        args.num_proc_other_threads
    };
    let nexus = Arc::new(Mutex::new(nexus));
    let args = Arc::new(args);
    let mut handles = Vec::with_capacity(num_threads);
    for i in 0..num_threads {
        let args = args.clone();
        let nexus = nexus.clone();
        let ctrl_c_pressed = ctrl_c_pressed.clone();
        let handle = thread::spawn(move || {
            thread_func(i, args, nexus, ctrl_c_pressed);
        });
        // TODO: bind_to_core
        handles.push(handle);
    }
    for handle in handles.drain(..) {
        handle.join().unwrap();
    }
    Ok(())
}
