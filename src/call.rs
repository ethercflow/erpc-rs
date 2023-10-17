// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::boxed::Box;

use async_channel::{bounded, Sender};
use erpc_sys::{c_int, c_void};

use crate::{
    buf::MsgBufferReader,
    channel::SubChannel,
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
    pub fn resolve(self, rpc: &mut Rpc) {
        match self {
            RpcCall::Call(call) => call.resolve(rpc),
            RpcCall::CallTag(tag) => tag.resolve(rpc),
        }
    }
}

/// A Call represents an RPC.
pub struct Call {
    pub sid: c_int,
    pub req_type: u8,
    pub req_msgbuf: &'static mut MsgBuffer,
    pub resp_msgbuf: &'static mut MsgBuffer,
    pub cb: ContFunc,
    pub tx: Sender<MsgBufferReader>,
}

unsafe impl Send for Call {}

impl Call {
    pub async fn unary<Req, Resp>(
        subchan: &SubChannel,
        method: &Method<Req, Resp>,
        req: &Req,
        req_msgbuf: &'static mut MsgBuffer,
        resp_msgbuf: &'static mut MsgBuffer,
        cb: ContFunc,
    ) -> Result<Resp> {
        // FIXME: channel should be provided by the client instead of temporarily creating it each time
        let (tx, rx) = bounded::<MsgBufferReader>(1);
        (method.req_ser())(req, req_msgbuf)?;
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

    pub fn resolve(self, rpc: &mut Rpc) {
        rpc.enqueue_request(
            self.sid,
            self.req_type,
            self.req_msgbuf,
            self.resp_msgbuf,
            self.cb,
            Some(Box::into_raw(Box::new(self.tx)) as *mut c_void),
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
