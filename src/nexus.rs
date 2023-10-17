// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::pin::Pin;

use crate::error::{Error, Result};
use erpc_sys::{
    c_void,
    erpc::{self, ReqHandle as RawReqHandle},
    UniquePtr, WithinUniquePtr, EEXIST, EINVAL, EPERM,
};

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
        let res = 0 - i32::from(unsafe {
            self.as_inner_mut().register_req_func(
                req_type,
                req_func as *mut c_void,
                erpc::ReqFuncType::kForeground,
            )
        });
        match res as u32 {
            0 => Ok(()),
            EPERM => Err(Error::Internal("registration not permitted".into())),
            EEXIST => Err(Error::Internal(format!(
                "handler for {req_type} already exists"
            ))),
            EINVAL => Err(Error::Internal("invalid handler".into())),
            _ => unreachable!(),
        }
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
