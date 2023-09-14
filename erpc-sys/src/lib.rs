// Copyright (c) 2023, IOMesh Inc. All rights reserved.

pub use autocxx::prelude::*;

include_cpp! {
    #include "erpc_wrapper.h"
    safety!(unsafe_ffi)
    generate_pod!("erpc::ChronoTimer")
    generate_pod!("erpc::FastRand")
    generate!("erpc::get_uri_for_process")
    generate!("erpc::kInvalidBgETid")
    generate!("erpc::kSessionCredits")
    generate!("erpc::kSessionReqWindow")
    generate!("erpc::kInvalidSessionNum")
    generate!("erpc::MsgBuffer")
    generate!("erpc::Nexus")
    generate!("erpc::ReqHandle")
    generate!("erpc::Rpc")
    generate!("erpc::SmEventType")
    generate!("erpc::SmErrType")
    generate!("erpc::Timely")
    generate!("erpc::TimingWheel")
    generate!("erpc::rdtsc")
    generate!("erpc::to_usec")
    generate!("erpc::sm_err_type_str")
    generate!("erpc::sm_event_type_str")
    generate!("erpc::wheel_record_t")
    block!("erpc::HugeAlloc")
}

pub use autocxx::moveit;
pub use cxx::CxxVector;
pub use ffi::*;

unsafe impl Send for erpc::Nexus {}
unsafe impl Sync for erpc::Nexus {}

unsafe impl Send for erpc::ReqHandle {}
unsafe impl Sync for erpc::ReqHandle {}
