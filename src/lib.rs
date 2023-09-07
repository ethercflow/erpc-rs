// Copyright (c) 2023, IOMesh Inc. All rights reserved.

mod error;
mod msg_buffer;
mod nexus;
mod req_handle;
mod rpc;

pub mod prelude {
    //! A "prelude" for crates using `erpc-rs`.
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
    pub use erpc_sys::erpc::ReqHandle as RawReqHandle;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::SmErrType;
    #[doc(no_inline)]
    pub use erpc_sys::erpc::SmEventType;
    #[doc(no_inline)]
    pub use erpc_sys::{c_void, c_int};

}
