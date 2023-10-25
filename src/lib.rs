// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(get_mut_unchecked)]
#![feature(sort_floats)]

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
mod server;
#[cfg(feature = "bench_stat")]
mod stat;
mod timely;
mod timing_wheel;

pub mod prelude {
    //! A "prelude" for crates using `erpc-rs`.
    #[doc(no_inline)]
    pub use crate::buf::MsgBufferReader;
    #[doc(no_inline)]
    pub use crate::call::{CallTag, Codec, RpcCall, Tag};
    #[doc(no_inline)]
    pub use crate::channel::{Channel, ChannelBuilder, ClientRpcContext};
    #[doc(no_inline)]
    pub use crate::client::Client;
    #[doc(no_inline)]
    pub use crate::codec::{pr_codec::de as pr_de, pr_codec::ser as pr_ser, Marshaller};
    #[doc(no_inline)]
    pub use crate::env::{EnvBuilder, Environment};
    #[doc(no_inline)]
    pub use crate::error::Result;
    #[doc(no_inline)]
    pub use crate::method::Method;
    #[doc(no_inline)]
    pub use crate::msg_buffer::MsgBuffer;
    #[doc(no_inline)]
    pub use crate::nexus::{Nexus, ReqHandler};
    #[doc(no_inline)]
    pub use crate::req_handle::ReqHandle;
    #[doc(no_inline)]
    pub use crate::rpc::{ContFunc, Rpc, SmHandler};
    #[doc(no_inline)]
    pub use crate::server::{Server, ServerBuilder, ServerRpcContext, Service, ServiceBuilder};
    #[doc(no_inline)]
    pub use crate::timely::Timely;
    #[doc(no_inline)]
    pub use crate::timing_wheel::TimingWheel;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::{
        get_uri_for_process, kSessionCredits, rdtsc, sm_err_type_str, sm_event_type_str, to_usec,
        ChronoTimer, FastRand, MsgBuffer as RawMsgBuffer, ReqHandle as RawReqHandle, SmErrType,
        SmEventType,
    };
    #[doc(no_inline)]
    pub use erpc_sys::{c_int, c_void, moveit, UniquePtr};
}
