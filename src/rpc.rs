// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use crate::msg_buffer::MsgBuffer;
use crate::nexus::Nexus;
use crate::req_handle::ReqHandle;
use erpc_sys::{
    c_int, c_void,
    erpc::{self, kInvalidBgETid, SmErrType, SmEventType},
    UniquePtr, WithinUniquePtr,
};
use std::{pin::Pin, ptr};

pub type SmHandler = extern "C" fn(c_int, SmEventType, SmErrType, *mut c_void);
pub type ContFunc = extern "C" fn(*mut c_void, *mut c_void);

pub struct Rpc {
    inner: UniquePtr<erpc::Rpc>,
}

impl Rpc {
    #[inline]
    pub fn new(
        nexus: &mut Nexus,
        context: Option<*mut c_void>,
        rpc_id: u8,
        sm_handler: Option<SmHandler>,
        phy_port: u8,
    ) -> Self {
        Rpc {
            inner: unsafe {
                erpc::Rpc::new(
                    nexus.as_inner_mut().get_unchecked_mut(),
                    context.unwrap_or(ptr::null_mut()),
                    rpc_id,
                    match sm_handler {
                        Some(h) => h as *mut c_void,
                        None => ptr::null_mut(),
                    },
                    phy_port,
                )
            }
            .within_unique_ptr(),
        }
    }

    #[inline]
    pub fn create_session(&mut self, remote_uri: &str, rem_rpc_id: u8) -> c_int {
        self.as_inner_mut().create_session(remote_uri, rem_rpc_id)
    }

    #[inline]
    pub fn is_connected(&self, session_num: c_int) -> bool {
        self.as_inner().is_connected(session_num)
    }

    #[inline]
    pub fn run_event_loop_once(&mut self) {
        self.as_inner_mut().run_event_loop_once();
    }

    #[inline]
    pub fn run_event_loop(&mut self, timeout_ms: usize) {
        self.as_inner_mut().run_event_loop(timeout_ms);
    }

    #[inline]
    pub fn alloc_msg_buffer_or_die(&mut self, max_data_size: usize) -> MsgBuffer {
        MsgBuffer {
            inner: self
                .inner
                .pin_mut()
                .alloc_msg_buffer_or_die(max_data_size)
                .within_unique_ptr(),
        }
    }

    #[inline]
    pub fn alloc_msg_buffer(&mut self, max_data_size: usize) -> MsgBuffer {
        MsgBuffer {
            inner: self
                .inner
                .pin_mut()
                .alloc_msg_buffer_or_die(max_data_size)
                .within_unique_ptr(),
        }
    }

    #[inline]
    pub fn enqueue_request(
        &mut self,
        session_num: c_int,
        req_type: u8,
        req_msgbuf: &mut MsgBuffer,
        resp_msgbuf: &mut MsgBuffer,
        cont_func: ContFunc,
        tag: Option<*mut c_void>,
    ) {
        unsafe {
            self.as_inner_mut().enqueue_request(
                session_num,
                req_type,
                req_msgbuf.as_inner_mut().get_unchecked_mut(),
                resp_msgbuf.as_inner_mut().get_unchecked_mut(),
                cont_func as *mut c_void,
                tag.unwrap_or(ptr::null_mut()),
                kInvalidBgETid,
            );
        }
    }

    #[inline]
    pub fn enqueue_response(&mut self, req_handle: &mut ReqHandle, resp_msgbuf: &mut MsgBuffer) {
        unsafe {
            self.as_inner_mut().enqueue_response(
                req_handle.as_inner_mut().get_unchecked_mut(),
                resp_msgbuf.as_inner_mut().get_unchecked_mut(),
            );
        }
    }

    #[inline]
    pub fn as_inner_mut(&mut self) -> Pin<&mut erpc::Rpc> {
        self.inner.pin_mut()
    }

    #[inline]
    pub fn as_inner(&self) -> &erpc::Rpc {
        &self.inner
    }
}
