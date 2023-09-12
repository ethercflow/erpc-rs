// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use erpc_sys::{
    c_void,
    erpc::{self, ReqHandle as RawReqHandle},
    UniquePtr, WithinUniquePtr,
};
use std::pin::Pin;

use anyhow::Result;

pub type ReqHandler = extern "C" fn(*mut RawReqHandle, *mut c_void);

pub struct Nexus {
    inner: UniquePtr<erpc::Nexus>,
}

unsafe impl Send for Nexus {}
unsafe impl Sync for Nexus {}

impl Nexus {
    #[inline]
    pub fn new(local_uri: &str, numa_node: usize) -> Self {
        Nexus {
            inner: erpc::Nexus::new(local_uri, numa_node, 0).within_unique_ptr(),
        }
    }

    #[inline]
    pub fn register_req_func(&mut self, req_type: u8, req_func: ReqHandler) -> Result<()> {
        unsafe {
            self.as_inner_mut().register_req_func(
                req_type,
                req_func as *mut c_void,
                erpc::ReqFuncType::kForeground,
            )
        };
        Ok(())
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut erpc::Nexus> {
        self.inner.pin_mut()
    }

    #[inline]
    pub fn as_inner(&self) -> &erpc::Nexus {
        &self.inner
    }
}
