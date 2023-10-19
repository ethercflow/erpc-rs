// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::sync::Arc;

use crate::{
    call::Call,
    channel::{Channel, SubChannel},
    error::Result,
    method::Method,
    msg_buffer::MsgBuffer,
    rpc::ContFunc,
};

/// A generic client for making RPC calls.
#[derive(Clone)]
pub struct Client {
    pub chan: SubChannel,
}

impl Client {
    /// Initialize a new [`Client`].
    pub fn new(mut channel: Channel) -> Self {
        Client {
            chan: channel.pick_subchan().unwrap(),
        }
    }

    /// Create an asynchronized unary RPC call.
    pub async fn unary_call<Req, Resp>(
        &self,
        method: &Method<Req, Resp>,
        req: &Req,
        req_msgbuf: Arc<MsgBuffer>,
        resp_msgbuf: Arc<MsgBuffer>,
        cb: ContFunc,
    ) -> Result<Resp> {
        Call::unary(&self.chan, method, req, req_msgbuf, resp_msgbuf, cb).await
    }

    pub fn alloc_msg_buffer(&mut self, max_data_size: usize) -> MsgBuffer {
        let rpc = unsafe { Arc::get_mut_unchecked(&mut self.chan.rpc) };
        rpc.alloc_msg_buffer_or_die(max_data_size)
    }
}
