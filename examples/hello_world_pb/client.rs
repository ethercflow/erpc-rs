// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(get_mut_unchecked)]

mod common;
mod helloworld;

use anyhow::Result;
use common::*;
use erpc_rs::prelude::*;
use helloworld::{GreeterClient, HelloRequest};
use std::{mem::MaybeUninit, sync::Arc, boxed::Box};
use async_channel::Sender;

static mut REQ: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();
static mut RESP: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();

extern "C" fn cont_func(_: *mut c_void, tag: *mut c_void) {
    let resp = unsafe { RESP.assume_init_mut() };
    let msg_buffer_reader = unsafe {MsgBufferReader::new(resp.as_inner() as *const RawMsgBuffer)} ;
    let tx = unsafe {Box::from_raw(tag as *mut Sender<MsgBufferReader>)};
    tx.send_blocking(msg_buffer_reader).unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let local_uri = K_CLIENT_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let server_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let env = Arc::new(EnvBuilder::new(local_uri).chan_count(1).build());
    let ch = ChannelBuilder::new(env, PHY_PORT)
        .subchan_count(1)
        .timeout_ms(0)
        .connect(&server_uri)
        .await
        .unwrap();
    let mut ch1 = ch.clone();
    let rpc = unsafe { Arc::get_mut_unchecked(&mut ch1.rpc) };
    let client = GreeterClient::new(ch);
    let req = HelloRequest { name: "world".to_owned() };

    unsafe {
        REQ.as_mut_ptr()
            .write(rpc.alloc_msg_buffer_or_die(K_MSG_SIZE));
        RESP.as_mut_ptr()
            .write(rpc.alloc_msg_buffer_or_die(K_MSG_SIZE));
    }

    let reply = unsafe {
        client
            .say_hello(
                &req,
                REQ.assume_init_mut(),
                RESP.assume_init_mut(),
                cont_func,
            )
            .await
    }
    .unwrap();
    println!("Greeter received: {}", reply.message);

    unsafe {
        REQ.assume_init_drop();
        RESP.assume_init_drop();
    }

    Ok(())
}
