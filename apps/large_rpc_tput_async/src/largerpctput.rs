#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BenchRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub buf: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BenchResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub buf: ::prost::alloc::vec::Vec<u8>,
}
pub const METHOD_BENCH_SEND_REQUEST: ::erpc_rs::prelude::Method<BenchRequest, BenchResponse> =
    ::erpc_rs::prelude::Method {
        id: 1,
        req_mar: ::erpc_rs::prelude::Marshaller {
            ser: ::erpc_rs::prelude::pr_ser,
            de: ::erpc_rs::prelude::pr_de,
        },
        resp_mar: ::erpc_rs::prelude::Marshaller {
            ser: ::erpc_rs::prelude::pr_ser,
            de: ::erpc_rs::prelude::pr_de,
        },
    };
#[derive(Clone)]
pub struct BenchClient {
    pub client: ::erpc_rs::prelude::Client,
}
impl BenchClient {
    pub fn new(channel: ::erpc_rs::prelude::Channel) -> Self {
        BenchClient {
            client: ::erpc_rs::prelude::Client::new(channel),
        }
    }
    pub async fn send_request(
        &self,
        req: &BenchRequest,
        req_msgbuf: std::sync::Arc<::erpc_rs::prelude::MsgBuffer>,
        resp_msgbuf: std::sync::Arc<::erpc_rs::prelude::MsgBuffer>,
        cb: ::erpc_rs::prelude::ContFunc,
        cid: usize,
    ) -> ::erpc_rs::prelude::Result<BenchResponse> {
        self.client
            .unary_call(
                &METHOD_BENCH_SEND_REQUEST,
                req,
                req_msgbuf,
                resp_msgbuf,
                cb,
                cid,
            )
            .await
    }
    pub fn alloc_msg_buffer(&mut self, max_data_size: usize) -> ::erpc_rs::prelude::MsgBuffer {
        self.client.alloc_msg_buffer(max_data_size)
    }
}
#[async_trait::async_trait]
pub trait Bench: Send + 'static {
    fn send_request(
        _req: ::erpc_rs::prelude::ReqHandle,
        _ctx: &'static mut ::erpc_rs::prelude::ServerRpcContext,
    ) {
        unimplemented!()
    }
    async fn send_request_async(
        _req: ::erpc_rs::prelude::ReqHandle,
        _rcp: std::sync::Arc<::erpc_rs::prelude::Rpc>,
        _tx: ::async_channel::Sender<::erpc_rs::prelude::RpcCall>,
        _codec: ::erpc_rs::prelude::Codec<BenchRequest, BenchResponse>,
    ) {
        unimplemented!()
    }
}
extern "C" fn send_request_wrapper<S: Bench>(
    req: *mut ::erpc_rs::prelude::RawReqHandle,
    ctx: *mut erpc_rs::prelude::c_void,
) {
    let result = std::panic::catch_unwind(|| {
        let req = erpc_rs::prelude::ReqHandle::from_inner_raw(req);
        let ctx = unsafe { &mut *(ctx as *mut ::erpc_rs::prelude::ServerRpcContext) };
        S::send_request(req, ctx);
    });
    if result.is_err() {
        std::process::abort();
    }
}
unsafe fn send_request_wrapper_into<S: Bench>() -> ::erpc_rs::prelude::ReqHandler {
    send_request_wrapper::<S>
}
pub fn create_bench<S: Bench + Send + Clone + 'static>() -> ::erpc_rs::prelude::Service {
    let mut builder = ::erpc_rs::prelude::ServiceBuilder::new();
    builder = builder.add_unary_handler(
        &METHOD_BENCH_SEND_REQUEST,
        move |req, rpc, tx, codec| std::boxed::Box::pin(S::send_request_async(req, rpc, tx, codec)),
        unsafe { send_request_wrapper_into::<S>() },
    );
    builder.build()
}
