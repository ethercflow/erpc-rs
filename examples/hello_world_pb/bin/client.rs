// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use anyhow::Result;
use erpc_rs::prelude::*;

use hello_world_pb::{
    common::*,
    helloworld::{GreeterClient, HelloRequest, METHOD_GREETER_SAY_HELLO},
};

// TODO: make sure if there's no other user-related logic, this function can be
// automatically generated and invisible to the user.
extern "C" fn cont_func(ctx: *mut c_void, tag: *mut c_void) {
    let ctx = unsafe { &mut *(ctx as *mut ClientRpcContext) };
    let tag = unsafe { Box::from_raw(tag as *mut Tag) };
    let (_, resp) = ctx
        .resp_msgbufs
        .get_mut(METHOD_GREETER_SAY_HELLO.id as usize)
        .unwrap()
        .remove_entry(&tag.idx)
        .unwrap();
    let msg_buffer_reader = unsafe { MsgBufferReader::new(resp.as_inner() as *const RawMsgBuffer) };
    tag.tx.send_blocking(msg_buffer_reader).unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let local_uri = K_CLIENT_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let server_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let env = Arc::new(EnvBuilder::new(local_uri).chan_count(1).build());
    let mut ch = ChannelBuilder::new(env, PHY_PORT)
        .subchan_count(1)
        .timeout_ms(0)
        .connect(&server_uri)
        .await
        .unwrap();
    let mut client = GreeterClient::new(ch.clone());
    let req = HelloRequest {
        name: "world".to_owned(),
    };

    let req_msgbuf = Arc::new(client.alloc_msg_buffer(K_MSG_SIZE));
    let resp_msgbuf = Arc::new(client.alloc_msg_buffer(K_MSG_SIZE));

    let reply = client
        .say_hello(&req, req_msgbuf.clone(), resp_msgbuf.clone(), cont_func)
        .await
        .unwrap();
    println!("Greeter received: {}", reply.message);

    ch.shutdown().await.unwrap();
    Ok(())
}
