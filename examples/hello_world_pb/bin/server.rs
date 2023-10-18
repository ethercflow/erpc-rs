// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{mem::MaybeUninit, sync::Arc};

use anyhow::Result;
use erpc_rs::prelude::*;
use tokio::signal;

use hello_world_pb::{
    common::*,
    helloworld::{create_greeter, Greeter, HelloReply, HelloRequest, METHOD_GREETER_SAY_HELLO},
};

static mut SERVER: MaybeUninit<Server> = MaybeUninit::uninit();

#[derive(Clone)]
struct GreeterService;

#[async_trait::async_trait]
impl Greeter for GreeterService {
    fn say_hello(
        req: ::erpc_rs::prelude::ReqHandle,
        ctx: &'static mut ::erpc_rs::prelude::RpcContext,
    ) {
        if let RpcContext::Server(sctx) = ctx {
            let f = unsafe { sctx.get_handler(METHOD_GREETER_SAY_HELLO.id) }
                .unwrap()
                .handle(req);
            sctx.spawn(f);
        }
    }
    async fn say_hello_async(
        mut req_handle: ::erpc_rs::prelude::ReqHandle,
        codec: ::erpc_rs::prelude::Codec<HelloRequest, HelloReply>,
    ) {
        let msg_buffer_reader = unsafe { MsgBufferReader::new(req_handle.get_req_msgbuf()) };
        let req = (codec.de)(msg_buffer_reader).unwrap();
        let msg = format!("Hello {}", req.name);
        let resp = HelloReply { message: msg };
        let mut resp_msgbuf = unsafe {
            // FIXME: c++ mutex will be called, may become a performance bottleneck in actual use
            SERVER.assume_init_mut().alloc_msg_buffer(K_MSG_SIZE)
        };
        (codec.ser)(&resp, &mut resp_msgbuf).unwrap();
        req_handle.init_dyn_resp_msgbuf_from_allocated(&mut resp_msgbuf);
        unsafe {
            SERVER
                .assume_init_mut()
                .ch
                .tx
                .send(RpcCall::CallTag(CallTag { req_handle }))
                .await
                .unwrap();
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let local_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;
    let env = Arc::new(EnvBuilder::new(local_uri).build());
    let service = create_greeter::<GreeterService>();
    unsafe {
        SERVER.as_mut_ptr().write(
            ServerBuilder::new(env, PHY_PORT, 0)
                .register_service(service)
                .build_and_start()
                .await
                .unwrap(),
        );
    }
    signal::ctrl_c().await?;
    eprintln!("Ctrl-c received!");
    Ok(())
}
