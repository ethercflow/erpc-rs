// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::call::Call;
use crate::error::Result;
use crate::method::Method;
use crate::{rpc::ContFunc, channel::{Channel, SubChannel}, msg_buffer::MsgBuffer};

/// A generic client for making RPC calls.
#[derive(Clone)]
pub struct Client {
    pub chan: SubChannel,
}

impl Client {
    /// Initialize a new [`Client`].
    pub fn new(mut channel: Channel) -> Self {
        Client { chan: channel.pick_subchan().unwrap() }
    }

    /// Create an asynchronized unary RPC call.
    pub async fn unary_call<Req, Resp>(
        &self,
        method: &Method<Req, Resp>,
        req: &Req,
        req_msgbuf: &'static mut MsgBuffer,
        resp_msgbuf: &'static mut MsgBuffer,
        cb: ContFunc,
    ) -> Result<Resp> {
        Call::unary(&self.chan, method, req, req_msgbuf, resp_msgbuf, cb).await
    }
}
