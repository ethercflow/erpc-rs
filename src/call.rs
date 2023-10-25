// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use async_channel::{bounded, Sender};
use erpc_sys::{c_int, c_void};

#[cfg(feature = "bench_stat")]
use erpc_sys::erpc::rdtsc;

use crate::{
    buf::MsgBufferReader,
    channel::{ClientRpcContext, SubChannel},
    codec::{DeserializeFn, SerializeFn},
    error::Result,
    method::Method,
    msg_buffer::MsgBuffer,
    req_handle::ReqHandle,
    rpc::{ContFunc, Rpc},
};

pub enum RpcCall {
    Call(Call),
    CallTag(CallTag),
}

impl RpcCall {
    pub fn resolve(self, rpc: &mut Rpc, ctx: *mut c_void) {
        match self {
            RpcCall::Call(call) => call.resolve(rpc, ctx),
            RpcCall::CallTag(tag) => tag.resolve(rpc),
        }
    }
}

pub struct Tag {
    pub tx: Sender<MsgBufferReader>,
    pub idx: u16,
}

/// A Call represents an RPC.
pub struct Call {
    pub sid: c_int,
    pub req_type: u8,
    pub req_msgbuf: Arc<MsgBuffer>,
    pub resp_msgbuf: Arc<MsgBuffer>,
    pub cb: ContFunc,
    pub tx: Sender<MsgBufferReader>,
}

unsafe impl Send for Call {}

impl Call {
    pub async fn unary<Req, Resp>(
        subchan: &SubChannel,
        method: &Method<Req, Resp>,
        req: &Req,
        mut req_msgbuf: Arc<MsgBuffer>,
        resp_msgbuf: Arc<MsgBuffer>,
        cb: ContFunc,
    ) -> Result<Resp> {
        let (tx, rx) = bounded::<MsgBufferReader>(1);
        (method.req_ser())(req, unsafe { Arc::get_mut_unchecked(&mut req_msgbuf) })?;
        subchan
            .tx
            .send(RpcCall::Call(Call {
                sid: subchan.id,
                req_type: method.id,
                req_msgbuf,
                resp_msgbuf,
                cb,
                tx,
            }))
            .await
            .unwrap();
        let resp = rx.recv().await.unwrap();
        (method.resp_de())(resp)
    }

    pub fn resolve(mut self, rpc: &mut Rpc, ctx: *mut c_void) {
        let ctx = unsafe { &mut *(ctx as *mut ClientRpcContext) };
        let idx = ctx
            .resp_msgbufs_idxs
            .get_mut(self.req_type as usize)
            .unwrap();
        *idx = idx.wrapping_add(1);
        let tag = Tag {
            tx: self.tx,
            idx: *idx,
        };
        let _ = ctx
            .resp_msgbufs
            .get_mut(self.req_type as usize)
            .unwrap()
            .insert(*idx, self.resp_msgbuf.clone());
        #[cfg(feature = "bench_stat")]
        {
            ctx.bench_stat.req_ts[*idx as usize % 32] = rdtsc();
        }
        rpc.enqueue_request(
            self.sid,
            self.req_type,
            unsafe { Arc::get_mut_unchecked(&mut self.req_msgbuf) },
            unsafe { Arc::get_mut_unchecked(&mut self.resp_msgbuf) },
            self.cb,
            Some(Box::into_raw(Box::new(tag)) as *mut c_void),
        );
        #[cfg(feature = "bench_stat")]
        {
            ctx.bench_stat.stat_tx_bytes_tot += ctx.bench_stat.args_req_size;
        }
    }
}

pub struct Codec<P, Q> {
    pub ser: SerializeFn<Q>,
    pub de: DeserializeFn<P>,
}

impl<P, Q> Codec<P, Q> {
    pub fn new(ser: SerializeFn<Q>, de: DeserializeFn<P>) -> Self {
        Codec { ser, de }
    }
}

pub struct CallTag {
    pub req_handle: ReqHandle,
}

impl CallTag {
    pub fn resolve(mut self, rpc: &mut Rpc) {
        let mut resp_msgbuf = self.req_handle.get_dyn_resp_msgbuf();
        rpc.enqueue_response(&mut self.req_handle, &mut resp_msgbuf);
    }
}
