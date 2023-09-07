// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::msg_buffer::MsgBuffer;
use erpc_sys::{erpc::ReqHandle as RawReqHandle, WithinUniquePtr};
use std::pin::Pin;

pub struct ReqHandle {
    inner: *mut RawReqHandle,
}

impl ReqHandle {
    #[inline]
    pub fn from_inner_raw(raw: *mut RawReqHandle) -> Self {
        ReqHandle { inner: raw }
    }

    #[inline]
    pub fn get_pre_resp_msgbuf(&mut self) -> MsgBuffer {
        MsgBuffer {
            inner: self
                .as_inner_mut()
                .get_pre_resp_msgbuf()
                .within_unique_ptr(),
        }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut RawReqHandle> {
        unsafe { Pin::new_unchecked(&mut *self.inner) }
    }
}
