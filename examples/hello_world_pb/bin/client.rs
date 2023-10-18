// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{boxed::Box, mem::MaybeUninit, sync::Arc};

use anyhow::Result;
use async_channel::Sender;
use erpc_rs::prelude::*;

use hello_world_pb::{
    common::*,
    helloworld::{GreeterClient, HelloRequest},
};

static mut REQ: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();
static mut RESP: MaybeUninit<MsgBuffer> = MaybeUninit::uninit();

extern "C" fn cont_func(_: *mut c_void, tag: *mut c_void) {
    let resp = unsafe { RESP.assume_init_mut() };
    let msg_buffer_reader = unsafe { MsgBufferReader::new(resp.as_inner() as *const RawMsgBuffer) };
    let tx = unsafe { Box::from_raw(tag as *mut Sender<MsgBufferReader>) };
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
    let mut client = GreeterClient::new(ch);
    let req = HelloRequest {
        name: "world".to_owned(),
    };

    unsafe {
        REQ.as_mut_ptr().write(client.alloc_msg_buffer(K_MSG_SIZE));
        RESP.as_mut_ptr().write(client.alloc_msg_buffer(K_MSG_SIZE));
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
