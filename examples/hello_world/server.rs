// Copyright (c) 2023, IOMesh Inc. All rights reserved.

mod common;

use common::*;
use erpc_rs::prelude::*;
use std::{
    mem::MaybeUninit,
    ptr::{self, copy_nonoverlapping},
};

static mut RPC: MaybeUninit<Rpc> = MaybeUninit::uninit();

extern "C" fn req_handler(req_handle: *mut RawReqHandle, _ctx: *mut c_void) {
    let mut req_handle = ReqHandle::from_inner_raw(req_handle);
    let mut resp_msgbuf = req_handle.get_pre_resp_msgbuf();
    resp_msgbuf.resize(K_MSG_SIZE);
    let buf = resp_msgbuf.get_inner_buf();
    let resp_msg = "hello".as_bytes();
    unsafe {
        copy_nonoverlapping(resp_msg.as_ptr(), buf, resp_msg.len());
        RPC.assume_init_mut()
            .enqueue_response(&mut req_handle, &mut resp_msgbuf);
    }
}

fn main() {
    let server_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let mut nexus = Nexus::new(&server_uri, 0);
    nexus.register_req_func(K_REQ_TYPE, req_handler).unwrap();
    unsafe {
        ptr::write(RPC.as_mut_ptr(), Rpc::new(&mut nexus, None, 0, None, 0));
        RPC.assume_init_mut().run_event_loop(100000);
        RPC.assume_init_drop();
    }
}
