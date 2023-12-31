// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(get_mut_unchecked)]

use std::sync::Arc;

use anyhow::Result;
use async_channel::Sender;
use erpc_rs::prelude::*;
use tokio::signal;

use hello_world_pb::{
    common::*,
    helloworld::{create_greeter, Greeter, HelloReply, HelloRequest, METHOD_GREETER_SAY_HELLO},
};

#[derive(Clone)]
struct GreeterService;

#[async_trait::async_trait]
impl Greeter for GreeterService {
    fn say_hello(req: ReqHandle, ctx: &'static mut ServerRpcContext) {
        let rpc = ctx.rpc.clone();
        let tx = ctx.tx.clone();
        let f = ctx
            .get_handler(METHOD_GREETER_SAY_HELLO.id)
            .unwrap()
            .handle(req, rpc, tx);
        ctx.spawn(f);
    }
    async fn say_hello_async(
        mut req_handle: ReqHandle,
        mut rpc: Arc<Rpc>,
        tx: Sender<RpcCall>,
        codec: Codec<HelloRequest, HelloReply>,
    ) {
        let msg_buffer_reader = unsafe { MsgBufferReader::new(req_handle.get_req_msgbuf()) };
        let req = (codec.de)(msg_buffer_reader).unwrap();
        let msg = format!("Hello {}", req.name);
        let resp = HelloReply { message: msg };
        let mut resp_msgbuf = unsafe {
            let rpc = Arc::get_mut_unchecked(&mut rpc);
            // FIXME: c++ mutex will be called, may become a performance bottleneck in actual use
            rpc.alloc_msg_buffer(K_MSG_SIZE)
        };
        (codec.ser)(&resp, &mut resp_msgbuf).unwrap();
        req_handle.init_dyn_resp_msgbuf_from_allocated(&mut resp_msgbuf);
        tx.send(RpcCall::CallTag(CallTag { req_handle }))
            .await
            .unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let local_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let env = Arc::new(EnvBuilder::new(local_uri).build());
    let service = create_greeter::<GreeterService>();
    let mut server = ServerBuilder::new(env, PHY_PORT, 0)
        .register_service(service)
        .build_and_start()
        .await
        .unwrap();
    signal::ctrl_c().await?;
    eprintln!("Ctrl-c received!");
    server.shutdown().await.unwrap();
    Ok(())
}
