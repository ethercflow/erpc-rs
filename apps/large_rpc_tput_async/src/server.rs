// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use async_channel::Sender;
use erpc_rs::prelude::*;
use tokio::signal;

use crate::{
    cli::Args,
    common::K_APP_EV_LOOP_MS,
    largerpctput::{create_bench, Bench, BenchRequest, BenchResponse, METHOD_BENCH_SEND_REQUEST},
};

#[derive(Clone)]
struct BenchService;

#[async_trait::async_trait]
impl Bench for BenchService {
    fn send_request(req: ReqHandle, ctx: &'static mut ServerRpcContext) {
        #[cfg(feature = "bench_stat")]
        {
            // TODO: support concurrency more than 1
            ctx.bench_stat.req_ts[0] = rdtsc();
        }
        let rpc = ctx.rpc.clone();
        let tx = ctx.tx.clone();
        let f = ctx
            .get_handler(METHOD_BENCH_SEND_REQUEST.id)
            .unwrap()
            .handle(req, rpc, tx);
        ctx.spawn(f);
    }

    async fn send_request_async(
        mut req_handle: ReqHandle,
        mut rpc: Arc<Rpc>,
        tx: Sender<RpcCall>,
        codec: Codec<BenchRequest, BenchResponse>,
    ) {
        let msg_buffer_reader = unsafe { MsgBufferReader::new(req_handle.get_req_msgbuf()) };
        let req = (codec.de)(msg_buffer_reader).unwrap();
        let resp_byte = req.buf[0];
        let mut resp_msgbuf = unsafe {
            let rpc = Arc::get_mut_unchecked(&mut rpc);
            // FIXME: c++ mutex will be called, may become a performance bottleneck in actual use
            rpc.alloc_msg_buffer(32 + 10)
        };
        let mut resp = BenchResponse { buf: vec![0; 32] };
        resp.buf[0] = resp_byte;
        (codec.ser)(&resp, &mut resp_msgbuf).unwrap();
        req_handle.init_dyn_resp_msgbuf_from_allocated(&mut resp_msgbuf);
        tx.send(RpcCall::CallTag(CallTag { req_handle }))
            .await
            .unwrap();
    }
}

pub async fn server_main(args: Args) -> Result<()> {
    let local_uri = (*get_uri_for_process(args.process_id)).to_string();
    let env = Arc::new(EnvBuilder::new(local_uri).build());
    let service = create_bench::<BenchService>();
    let mut server = ServerBuilder::new(env, args.phy_port, K_APP_EV_LOOP_MS)
        .register_service(service)
        .build_and_start()
        .await
        .unwrap();
    signal::ctrl_c().await.unwrap();
    eprintln!("Ctrl-c received!");
    server.shutdown().await.unwrap();

    Ok(())
}
