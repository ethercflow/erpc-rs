// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::msg_buffer::MsgBuffer;
use erpc_sys::{erpc::ReqHandle as RawReqHandle, erpc::MsgBuffer as RawMsgBuffer, WithinUniquePtr};
use std::pin::Pin;

pub struct ReqHandle {
    inner: *mut RawReqHandle,
}

unsafe impl Send for ReqHandle {}
unsafe impl Sync for ReqHandle {}

impl ReqHandle {
    #[inline]
    pub fn from_inner_raw(raw: *mut RawReqHandle) -> Self {
        ReqHandle { inner: raw }
    }

    #[inline]
    pub fn get_req_msgbuf(&mut self) -> *const RawMsgBuffer {
        self.as_inner_mut().get_req_msgbuf()
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
    pub fn get_dyn_resp_msgbuf(&mut self) -> MsgBuffer {
        MsgBuffer {
            inner: self
                .as_inner_mut()
                .get_dyn_resp_msgbuf()
                .within_unique_ptr(),
        }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut RawReqHandle> {
        unsafe { Pin::new_unchecked(&mut *self.inner) }
    }
}
