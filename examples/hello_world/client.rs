// Copyright (c) 2023, IOMesh Inc. All rights reserved.

extern crate libc;

mod common;

use common::*;
use erpc_rs::prelude::*;
use libc::c_char;
use std::{ffi::CStr, mem::MaybeUninit, ptr};

static mut REQ: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();
static mut RESP: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();

extern "C" fn cont_func(_: *mut c_void, _: *mut c_void) {
    let resp = unsafe { RESP.assume_init_mut() };
    let buf = resp.get_inner_buf();
    let c_str = unsafe { CStr::from_ptr(buf as *const c_char) };
    let s = c_str.to_str().unwrap();
    println!("{}", s);
}

extern "C" fn sm_handler(_: c_int, _: SmEventType, _: SmErrType, _: *mut c_void) {}

fn main() {
    let client_uri = K_CLIENT_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let mut nexus = Nexus::new(&client_uri, 0);
    let mut rpc = Rpc::new(&mut nexus, None, 0, Some(sm_handler), 0);
    let server_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let session_num = rpc.create_session(&server_uri, 0).unwrap();
    loop {
        if rpc.is_connected(session_num) {
            break;
        }
        rpc.run_event_loop_once();
    }
    unsafe {
        ptr::write(REQ.as_mut_ptr(), rpc.alloc_msg_buffer_or_die(K_MSG_SIZE));
        ptr::write(RESP.as_mut_ptr(), rpc.alloc_msg_buffer_or_die(K_MSG_SIZE));
        rpc.enqueue_request(
            session_num,
            K_REQ_TYPE,
            REQ.assume_init_mut(),
            RESP.assume_init_mut(),
            cont_func,
            None,
        );
    }
    rpc.run_event_loop(100);
    unsafe {
        REQ.assume_init_drop();
        RESP.assume_init_drop();
    }
}
