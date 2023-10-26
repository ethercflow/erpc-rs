// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use erpc_rs::prelude::*;
use tokio::signal;

use crate::{
    cli::Args,
    common::*,
    largerpctput::{BenchClient, BenchRequest, METHOD_BENCH_SEND_REQUEST},
};

extern "C" fn cont_func(ctx: *mut c_void, tag: *mut c_void) {
    let ctx = unsafe { &mut *(ctx as *mut ClientRpcContext) };
    let tag = unsafe { Box::from_raw(tag as *mut Tag) };
    let (_, resp) = ctx
        .resp_msgbufs
        .get_mut(METHOD_BENCH_SEND_REQUEST.id as usize)
        .unwrap()
        .remove_entry(&tag.idx)
        .unwrap();
    let usec = to_usec(
        rdtsc() - ctx.bench_stat.req_ts[tag.cid],
        ctx.rpc.as_ref().unwrap().get_freq_ghz(),
    );
    ctx.bench_stat.lat_vec.push(usec);
    let msg_buffer_reader = unsafe { MsgBufferReader::new(resp.as_inner() as *const RawMsgBuffer) };
    tag.tx.send_blocking(msg_buffer_reader).unwrap();
    ctx.bench_stat.stat_rx_bytes_tot += ctx.bench_stat.args_resp_size;
}

pub async fn client_main(args: Args) -> Result<()> {
    let local_uri = (*get_uri_for_process(args.process_id)).to_string();
    let server_uri = (*get_uri_for_process(0)).to_string();
    let threads_nr = args.num_proc_other_threads;
    let env = Arc::new(EnvBuilder::new(local_uri).chan_count(threads_nr).build());

    let mut chs = Vec::new();

    for _i in 0..threads_nr {
        let ch = ChannelBuilder::new(env.clone(), args.phy_port)
            .subchan_count(1)
            .timeout_ms(K_APP_EV_LOOP_MS)
            .req_size(args.req_size)
            .resp_size(args.resp_size)
            .connect(server_uri.clone())
            .await
            .unwrap();
        let mut client = BenchClient::new(ch.clone());

        let mut req_msgbufs = Vec::new();
        let mut resp_msgbufs = Vec::new();
        let mut req = BenchRequest {
            buf: vec![0; args.req_size],
        };
        req.buf[0] = K_APP_DATA_BYTE;

        for j in 0..args.concurrency {
            // FIXME: +10 is to prevent the problem of insufficient space when pb encoding and decoding
            let req_msgbuf = Arc::new(client.alloc_msg_buffer(args.req_size + 10));
            let resp_msgbuf = Arc::new(client.alloc_msg_buffer(args.resp_size + 10));

            if req_msgbuf.get_data_size() != args.req_size + 10 {
                panic!("allocated req_msgbuf's data size not eq arg's req_size");
            }

            req_msgbufs.push(req_msgbuf.clone());
            resp_msgbufs.push(resp_msgbuf.clone());

            let client = client.clone();
            let req = req.clone();

            tokio::spawn(async move {
                loop {
                    let resp = client
                        .send_request(&req, req_msgbuf.clone(), resp_msgbuf.clone(), cont_func, j)
                        .await
                        .unwrap();
                    if resp.buf.len() != args.resp_size {
                        panic!("received resp's size not eq to arg's resp_size");
                    }
                    if resp.buf[0] != K_APP_DATA_BYTE {
                        panic!("Invalid resp data");
                    }
                }
            });
        }

        chs.push(ch);
    }
    signal::ctrl_c().await.unwrap();
    eprintln!("Ctrl-c received!");
    for mut ch in chs {
        ch.shutdown().await.unwrap();
    }
    Ok(())
}
