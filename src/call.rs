// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use async_channel::Sender;
use erpc_sys::{c_int, c_void};

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

/// A Call represents an RPC.
pub struct Call {
    pub sid: c_int,
    pub req_type: u8,
    pub req_msgbuf: Arc<MsgBuffer>,
    pub resp_msgbuf: Arc<MsgBuffer>,
    pub cb: ContFunc,
    pub tx: *mut Sender<MsgBufferReader>,
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
        (method.req_ser())(req, unsafe { Arc::get_mut_unchecked(&mut req_msgbuf) })?;
        subchan
            .tx
            .send(RpcCall::Call(Call {
                sid: subchan.id,
                req_type: method.id,
                req_msgbuf,
                resp_msgbuf,
                cb,
                tx: subchan.mbr_tx,
            }))
            .await
            .unwrap();
        let resp = subchan.mbr_rx.recv().await.unwrap();
        (method.resp_de())(resp)
    }

    pub fn resolve(mut self, rpc: &mut Rpc, ctx: *mut c_void) {
        unsafe {
            let ctx = &mut *(ctx as *mut ClientRpcContext);
            (*ctx)
                .resp_msgbufs
                .get_mut(self.req_type as usize)
                .unwrap()
                .push_back(self.resp_msgbuf.clone());
        }
        rpc.enqueue_request(
            self.sid,
            self.req_type,
            unsafe { Arc::get_mut_unchecked(&mut self.req_msgbuf) },
            unsafe { Arc::get_mut_unchecked(&mut self.resp_msgbuf) },
            self.cb,
            Some(self.tx as *mut c_void),
        );
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
