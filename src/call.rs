// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use async_channel::{bounded, Sender};
use erpc_sys::c_int;

use crate::{
    buf::MsgBufferReader, channel::SubChannel, error::Result, method::Method, prelude::MsgBuffer,
    rpc::{ContFunc, Rpc},
};

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
        let (tx, rx) = bounded::<MsgBufferReader>(1);
        (method.req_ser())(req, req_msgbuf)?;
        subchan.tx.send(Call {
            sid: subchan.id,
            req_type: method.id,
            req_msgbuf,
            resp_msgbuf,
            cb,
            tx,
        }).await.unwrap();
        let resp = rx.recv().await.unwrap();
        (method.resp_de())(resp)
    }

    pub fn resolve(&mut self, rpc: &mut Rpc) {
        rpc.enqueue_request(
            self.sid,
            self.req_type,
            self.req_msgbuf,
            self.resp_msgbuf,
            self.cb,
            None,
        )
    }
}
