// Copyright (c) 2023, IOMesh Inc. All rights reserved.

pub use autocxx::prelude::*;

include_cpp! {
    #include "erpc_wrapper.h"
    safety!(unsafe)
    generate!("erpc::SmEventType")
    generate!("erpc::SmErrType")
    generate!("erpc::ReqHandle")
    block!("erpc::HugeAlloc")
    generate!("erpc::MsgBuffer")
    generate!("erpc::Nexus")
    generate!("erpc::Rpc")
    generate!("erpc::kInvalidBgETid")
}

pub use ffi::*;
