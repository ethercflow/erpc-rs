// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{pin::Pin, ptr};

use erpc_sys::{
    c_int, c_void,
    erpc::{self, kInvalidBgETid, SmErrType, SmEventType},
    UniquePtr, WithinUniquePtr, EALREADY, EBUSY, EINVAL, ENOMEM, EPERM,
};

use crate::{
    error::{Error, Result},
    msg_buffer::MsgBuffer,
    nexus::Nexus,
    req_handle::ReqHandle,
    timely::Timely,
    timing_wheel::TimingWheel,
};

pub type SmHandler = extern "C" fn(c_int, SmEventType, SmErrType, *mut c_void);
pub type ContFunc = extern "C" fn(*mut c_void, *mut c_void);

pub struct Rpc {
    inner: UniquePtr<erpc::Rpc>,
}

/// Notice: Only MsgBuffer related ops are thread safe
unsafe impl Send for Rpc {}
unsafe impl Sync for Rpc {}

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
    pub fn create_session(&mut self, remote_uri: &str, rem_rpc_id: u8) -> Result<c_int> {
        let sid = self.as_inner_mut().create_session(remote_uri, rem_rpc_id);
        if i32::from(sid) >= 0 {
            return Ok(sid);
        }
        match (0 - i32::from(sid)) as u32 {
            EPERM => Err(Error::Internal("can't create session from a thread other than the one that created this Rpc object".into())),
            EINVAL => Err(Error::Internal("invalid remote hostname or remote Rpc is same as local".into())),
            ENOMEM => Err(Error::Internal("ring buffers exhausted".into())),
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn destroy_session(&mut self, session_num: c_int) -> Result<()> {
        let res = self.as_inner_mut().destroy_session(session_num);
        match (0 - i32::from(res)) as u32 {
            EPERM => Err(Error::Internal("can't destroy session from a thread other than the one that created this Rpc object or session already destroyed or session connection in progress.".into())),
            EINVAL => Err(Error::Internal("invalid session number or it's a server session.".into())),
            EBUSY => Err(Error::Internal("session has pending RPC requests.".into())),
            EALREADY => Err(Error::Internal("session disconnection in progress.".into())),
            _ => unreachable!(),
        }
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
    pub fn free_msg_buffer(&mut self, msg_buffer: &MsgBuffer) {
        self.inner.pin_mut().free_msg_buffer(msg_buffer.as_inner());
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
    pub fn get_timely(&mut self, session_num: c_int) -> Timely {
        Timely::from_inner_raw(self.as_inner_mut().get_timely(session_num))
    }

    #[inline]
    pub fn get_bandwidth(&self) -> usize {
        self.as_inner().get_bandwidth()
    }

    #[inline]
    pub fn get_freq_ghz(&self) -> f64 {
        self.as_inner().get_freq_ghz()
    }

    #[inline]
    pub fn get_rpc_id(&self) -> u8 {
        self.as_inner().get_rpc_id()
    }

    #[inline]
    pub fn get_wheel(&mut self) -> TimingWheel {
        TimingWheel::from_inner_raw(self.as_inner_mut().get_wheel())
    }

    #[inline]
    pub fn get_num_re_tx(&self, session_num: c_int) -> usize {
        self.as_inner().get_num_re_tx(session_num)
    }

    #[inline]
    pub fn reset_num_re_tx(&mut self, session_num: c_int) {
        self.as_inner_mut().reset_num_re_tx(session_num);
    }

    #[inline]
    pub fn sec_since_creation(&mut self) -> f64 {
        self.as_inner_mut().sec_since_creation()
    }

    #[inline]
    pub fn force_retry_connect_on_invalid_rpc_id(&mut self) {
        self.as_inner_mut().force_retry_connect_on_invalid_rpc_id();
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
