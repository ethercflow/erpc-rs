// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use erpc_sys::{erpc, UniquePtr};
use std::pin::Pin;

pub struct MsgBuffer {
    pub(crate) inner: UniquePtr<erpc::MsgBuffer>,
}

impl MsgBuffer {
    #[inline]
    pub fn get_inner_buf(&self) -> *mut u8 {
        self.as_inner().get_inner_buf()
    }

    #[inline]
    pub fn get_data_size(&self) -> usize {
        self.as_inner().get_data_size()
    }

    #[inline]
    pub fn resize(&mut self, new_data_size: usize) {
        unsafe {
            erpc::Rpc::resize_msg_buffer(self.as_inner_mut().get_unchecked_mut(), new_data_size);
        }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut erpc::MsgBuffer> {
        self.inner.pin_mut()
    }

    #[inline]
    pub fn as_inner(&self) -> &erpc::MsgBuffer {
        &self.inner
    }
}
