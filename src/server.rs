// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{
    collections::HashMap,
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
};

use async_channel::{unbounded, Sender, TryRecvError};
use erpc_sys::{
    c_int, c_void,
    erpc::{SmErrType, SmEventType},
};
use tokio::runtime::Runtime;

use crate::{
    call::{Codec, RpcCall},
    channel::Channel,
    codec::{DeserializeFn, SerializeFn},
    env::Environment,
    error::Result,
    method::Method,
    nexus::{Nexus, ReqHandler},
    req_handle::ReqHandle,
    rpc::Rpc,
};

pub type AsyncReqHandler = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// An RPC call holder.
#[derive(Clone)]
pub struct Handler<F> {
    cb: F,
}

impl<F> Handler<F> {
    pub fn new(cb: F) -> Handler<F> {
        Handler { cb }
    }
}

pub trait CloneableHandler: Send {
    fn handle(&mut self, req: ReqHandle) -> AsyncReqHandler;
    fn box_clone(&self) -> Box<dyn CloneableHandler>;
}

impl<F: 'static> CloneableHandler for Handler<F>
where
    F: FnMut(ReqHandle) -> AsyncReqHandler + Send + Clone,
{
    #[inline]
    fn handle(&mut self, req: ReqHandle) -> AsyncReqHandler {
        (self.cb)(req)
    }

    #[inline]
    fn box_clone(&self) -> Box<dyn CloneableHandler> {
        Box::new(self.clone())
    }
}

pub type BoxHandler = Box<dyn CloneableHandler>;

pub struct ServerRpcContext {
    registry: HashMap<u8, BoxHandler>,
    rpc: MaybeUninit<Arc<Rpc>>,
    pub rt: Runtime,
}

impl ServerRpcContext {
    #[inline]
    pub unsafe fn get_handler(&mut self, req_type: u8) -> Option<&mut BoxHandler> {
        self.registry.get_mut(&req_type)
    }

    #[inline]
    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.rt.spawn(f);
    }
}

extern "C" fn sm_handler(_: c_int, _: SmEventType, _: SmErrType, _: *mut c_void) {}

/// [`Service`] factory in order to configure the properties.
///
/// Use it to build a service which can be registered to a server.
pub struct ServiceBuilder {
    handlers: HashMap<u8, BoxHandler>,
    raw_handlers: HashMap<u8, ReqHandler>,
}

impl ServiceBuilder {
    /// Initialize a new [`ServiceBuilder`].
    pub fn new() -> Self {
        ServiceBuilder {
            handlers: HashMap::new(),
            raw_handlers: HashMap::new(),
        }
    }

    /// Add a unary RPC call handler.
    pub fn add_unary_handler<Req, Resp, F>(
        mut self,
        method: &Method<Req, Resp>,
        mut handler: F,
        raw_handler: ReqHandler,
    ) -> ServiceBuilder
    where
        Req: 'static,
        Resp: 'static,
        F: FnMut(ReqHandle, Codec<Req, Resp>) -> AsyncReqHandler + Send + Clone + 'static,
    {
        let (ser, de) = (method.resp_ser(), method.req_de());
        let h =
            move |req: ReqHandle| -> AsyncReqHandler { execute_unary(ser, de, req, &mut handler) };
        let ch = Box::new(Handler::new(h));
        self.handlers.insert(method.id, ch);
        self.raw_handlers.insert(method.id, raw_handler);
        self
    }

    /// Finalize the [`ServiceBuilder`] and build the [`Service`].
    pub fn build(self) -> Service {
        Service {
            handlers: self.handlers,
            raw_handlers: self.raw_handlers,
        }
    }
}

/// A eRPC service.
///
/// Use [`ServiceBuilder`] to build a [`Service`].
pub struct Service {
    handlers: HashMap<u8, BoxHandler>,
    raw_handlers: HashMap<u8, ReqHandler>,
}

/// [`Server`] factory in order to configure the properties.
pub struct ServerBuilder {
    env: Arc<Environment>,
    phy_port: u8,
    timeout_ms: usize,
    handlers: HashMap<u8, BoxHandler>,
    raw_handlers: HashMap<u8, ReqHandler>,
}

impl ServerBuilder {
    /// Initialize a new [`ServerBuilder`].
    pub fn new(env: Arc<Environment>, phy_port: u8, timeout_ms: usize) -> ServerBuilder {
        ServerBuilder {
            env,
            phy_port,
            timeout_ms,
            handlers: HashMap::new(),
            raw_handlers: HashMap::new(),
        }
    }

    /// Register a service.
    pub fn register_service(mut self, service: Service) -> ServerBuilder {
        self.handlers.extend(service.handlers);
        self.raw_handlers.extend(service.raw_handlers);
        self
    }

    /// Finalize the [`ServerBuilder`] and build the [`Server`].
    pub async fn build_and_start(self) -> Result<Server> {
        let env = self.env.pick_channel_env().unwrap();
        env.0
            .send(Box::new(
                move |id: u8, nexus: &mut Arc<Nexus>, chan_tx: Sender<Channel>| {
                    for (k, v) in &self.raw_handlers {
                        unsafe { Arc::get_mut_unchecked(nexus) }
                            .register_req_func(k.to_owned(), v.to_owned())
                            .unwrap();
                    }
                    let mut ctx = ServerRpcContext {
                        registry: self
                            .handlers
                            .iter()
                            .map(|(k, v)| (k.to_owned(), v.box_clone()))
                            .collect(),
                        rpc: MaybeUninit::uninit(),
                        rt: tokio::runtime::Runtime::new().unwrap(),
                    };
                    let (tx, rx) = unbounded::<RpcCall>();
                    let mut rpc = Arc::new(Rpc::new(
                        unsafe { Arc::get_mut_unchecked(nexus) },
                        Some(&mut ctx as *mut ServerRpcContext as *mut c_void),
                        id,
                        Some(sm_handler),
                        self.phy_port,
                    ));
                    unsafe { ctx.rpc.as_mut_ptr().write(rpc.clone()) };
                    let rpc_clone = rpc.clone();
                    let rpc = unsafe { Arc::get_mut_unchecked(&mut rpc) };
                    let chan = Channel {
                        subchans: Vec::default(),
                        assigned_idx: Arc::new(AtomicUsize::new(0)),
                        rpc: rpc_clone,
                        tx,
                    };
                    chan_tx.send_blocking(chan).unwrap();

                    loop {
                        match rx.try_recv() {
                            Ok(call) => call.resolve(rpc),
                            Err(TryRecvError::Empty) => {}
                            Err(TryRecvError::Closed) => {}
                        }
                        rpc.run_event_loop(self.timeout_ms);
                    }
                },
            ))
            .await
            .unwrap();

        Ok(Server {
            env: self.env,
            ch: env.1.recv().await.unwrap(),
        })
    }
}

#[allow(dead_code)]
pub struct Server {
    env: Arc<Environment>,
    pub ch: Channel,
}

// helper function to call a unary handler.
pub fn execute_unary<P, Q, F>(
    ser: SerializeFn<Q>,
    de: DeserializeFn<P>,
    req_handle: ReqHandle,
    f: &mut F,
) -> AsyncReqHandler
where
    F: FnMut(ReqHandle, Codec<P, Q>) -> AsyncReqHandler + Send + Clone,
{
    f(req_handle, Codec::new(ser, de))
}
