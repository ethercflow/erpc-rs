// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(get_mut_unchecked)]

mod buf;
mod call;
mod channel;
mod client;
mod codec;
mod env;
mod error;
mod method;
mod msg_buffer;
mod nexus;
mod req_handle;
mod rpc;
mod timely;
mod timing_wheel;

pub mod prelude {
    //! A "prelude" for crates using `erpc-rs`.
    #[doc(no_inline)]
    pub use crate::buf::MsgBufferReader;
    #[doc(no_inline)]
    pub use crate::channel::{Channel, ChannelBuilder};
    #[doc(no_inline)]
    pub use crate::client::Client;
    #[doc(no_inline)]
    pub use crate::codec::*;
    #[doc(no_inline)]
    pub use crate::env::{EnvBuilder, Environment};
    #[doc(no_inline)]
    pub use crate::msg_buffer::MsgBuffer;
    #[doc(no_inline)]
    pub use crate::nexus::Nexus;
    #[doc(no_inline)]
    pub use crate::nexus::ReqHandler;
    #[doc(no_inline)]
    pub use crate::req_handle::ReqHandle;
    #[doc(no_inline)]
    pub use crate::rpc::ContFunc;
    #[doc(no_inline)]
    pub use crate::rpc::Rpc;
    #[doc(no_inline)]
    pub use crate::rpc::SmHandler;
    #[doc(no_inline)]
    pub use crate::timely::Timely;
    #[doc(no_inline)]
    pub use crate::timing_wheel::TimingWheel;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::get_uri_for_process;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::kSessionCredits;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::rdtsc;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::sm_err_type_str;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::sm_event_type_str;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::to_usec;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::ChronoTimer;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::FastRand;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::MsgBuffer as RawMsgBuffer;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::ReqHandle as RawReqHandle;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::SmErrType;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::SmEventType;
    #[doc(no_inline)]
    pub use erpc_sys::{c_int, c_void, moveit, UniquePtr};
}
