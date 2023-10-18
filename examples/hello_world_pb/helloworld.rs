/// The request message containing the user's name.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloRequest {
    #[prost(string, tag = "1")]
    pub name: ::prost::alloc::string::String,
}
/// The response message containing the greetings
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct HelloReply {
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
}
pub const METHOD_GREETER_SAY_HELLO: ::erpc_rs::prelude::Method<HelloRequest, HelloReply> =
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
pub struct GreeterClient {
    pub client: ::erpc_rs::prelude::Client,
}
impl GreeterClient {
    pub fn new(channel: ::erpc_rs::prelude::Channel) -> Self {
        GreeterClient {
            client: ::erpc_rs::prelude::Client::new(channel),
        }
    }
    pub async fn say_hello(
        &self,
        req: &HelloRequest,
        req_msgbuf: &'static mut ::erpc_rs::prelude::MsgBuffer,
        resp_msgbuf: &'static mut ::erpc_rs::prelude::MsgBuffer,
        cb: ::erpc_rs::prelude::ContFunc,
    ) -> ::erpc_rs::prelude::Result<HelloReply> {
        self.client
            .unary_call(&METHOD_GREETER_SAY_HELLO, req, req_msgbuf, resp_msgbuf, cb)
            .await
    }
    pub fn alloc_msg_buffer(&mut self, max_data_size: usize) -> ::erpc_rs::prelude::MsgBuffer {
        self.client.alloc_msg_buffer(max_data_size)
    }
}
#[async_trait::async_trait]
pub trait Greeter: Send + 'static {
    fn say_hello(
        _req: ::erpc_rs::prelude::ReqHandle,
        _ctx: &'static mut ::erpc_rs::prelude::RpcContext,
    ) {
        unimplemented!()
    }
    async fn say_hello_async(
        _req: ::erpc_rs::prelude::ReqHandle,
        _codec: ::erpc_rs::prelude::Codec<HelloRequest, HelloReply>,
    ) {
        unimplemented!()
    }
}
extern "C" fn say_hello_wrapper<S: Greeter>(
    req: *mut ::erpc_rs::prelude::RawReqHandle,
    ctx: *mut erpc_rs::prelude::c_void,
) {
    let result = std::panic::catch_unwind(|| {
        let req = erpc_rs::prelude::ReqHandle::from_inner_raw(req);
        let ctx = unsafe { &mut *(ctx as *mut ::erpc_rs::prelude::RpcContext) };
        S::say_hello(req, ctx);
    });
    if result.is_err() {
        std::process::abort();
    }
}
unsafe fn say_hello_wrapper_into<S: Greeter>() -> ::erpc_rs::prelude::ReqHandler {
    say_hello_wrapper::<S>
}
pub fn create_greeter<S: Greeter + Send + Clone + 'static>() -> ::erpc_rs::prelude::Service {
    let mut builder = ::erpc_rs::prelude::ServiceBuilder::new();
    builder = builder.add_unary_handler(
        &METHOD_GREETER_SAY_HELLO,
        move |req, codec| std::boxed::Box::pin(S::say_hello_async(req, codec)),
        unsafe { say_hello_wrapper_into::<S>() },
    );
    builder.build()
}
