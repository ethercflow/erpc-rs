// Copyright (c) 2023, IOMesh Inc. All rights reserved.

extern crate num_cpus;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread::{Builder as ThreadBuilder, JoinHandle},
};

use async_channel::{bounded, Receiver, Sender};

use crate::{
    channel::{Channel, RpcPollFn},
    nexus::Nexus,
};

// event loop
fn poll_channel(id: u8, mut nexus: Arc<Nexus>, rx: Receiver<RpcPollFn>, tx: Sender<Channel>) {
    if let Ok(rpc_poll_fn) = rx.recv_blocking() {
        rpc_poll_fn(id, &mut nexus, tx);
    }
}

/// [`Environment`] factory in order to configure the properties.
pub struct EnvBuilder {
    /// local_uri A URI for this process formatted as hostname:udp_port.
    /// This hostname and UDP port correspond to the "control" network interface of
    /// the host, which eRPC uses for non-performance-critical session handshakes
    /// and management traffic. This is different from the fast "datapath" network
    /// interface that eRPC uses for performance-critical RPC traffic.
    local_uri: String,
    chan_count: usize,
    name_prefix: Option<String>,
    after_start: Option<Arc<dyn Fn() + Send + Sync>>,
    before_stop: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl EnvBuilder {
    /// Initialize a new [`EnvBuilder`].
    pub fn new<S: Into<String>>(local_uri: S) -> Self {
        EnvBuilder {
            local_uri: local_uri.into(),
            chan_count: num_cpus::get(),
            name_prefix: None,
            after_start: None,
            before_stop: None,
        }
    }

    /// Set the number of channels and polling threads. Each thread polls
    /// one channel.
    ///
    /// # Panics
    ///
    /// This method will panic if `count` is 0.
    pub fn chan_count(mut self, count: usize) -> EnvBuilder {
        assert!(count > 0);
        self.chan_count = count;
        self
    }

    /// Set the thread name prefix of each polling thread.
    pub fn name_prefix<S: Into<String>>(mut self, prefix: S) -> EnvBuilder {
        self.name_prefix = Some(prefix.into());
        self
    }

    /// Execute function `f` after each thread is started but before it starts doing work.
    pub fn after_start<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> EnvBuilder {
        self.after_start = Some(Arc::new(f));
        self
    }

    /// Execute function `f` before each thread stops.
    pub fn before_stop<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> EnvBuilder {
        self.before_stop = Some(Arc::new(f));
        self
    }

    /// Finalize the [`EnvBuilder`], build the [`Environment`] and initialize the gRPC library.
    pub fn build(self) -> Environment {
        let nexus = Arc::new(Nexus::new(self.local_uri.as_str(), 0));
        let mut handles = Vec::with_capacity(self.chan_count);
        let mut chs = Vec::with_capacity(self.chan_count);
        for i in 0..self.chan_count {
            let (tx, rx) = bounded::<RpcPollFn>(1);
            let (chan_tx, chan_rx) = bounded::<Channel>(1);
            let nexus = nexus.clone();
            let mut builder = ThreadBuilder::new();
            if let Some(ref prefix) = self.name_prefix {
                builder = builder.name(format!("{prefix}-{i}"));
            }
            let after_start = self.after_start.clone();
            let before_stop = self.before_stop.clone();
            let handle = builder
                .spawn(move || {
                    if let Some(f) = after_start {
                        f();
                    }
                    poll_channel(i.try_into().unwrap(), nexus, rx, chan_tx);
                    if let Some(f) = before_stop {
                        f();
                    }
                })
                .unwrap();
            handles.push(handle);
            chs.push((tx, chan_rx));
        }

        Environment {
            chs,
            idx: AtomicUsize::new(0),
            _handles: handles,
        }
    }
}

pub struct Environment {
    chs: Vec<(Sender<RpcPollFn>, Receiver<Channel>)>,
    idx: AtomicUsize,
    _handles: Vec<JoinHandle<()>>,
}

impl Environment {
    pub fn new(chan_count: usize, local_uri: String) -> Self {
        EnvBuilder::new(local_uri)
            .name_prefix("erpc-poll")
            .chan_count(chan_count)
            .build()
    }

    pub fn pick_channel_env(&self) -> Option<(Sender<RpcPollFn>, Receiver<Channel>)> {
        let idx = self.idx.fetch_add(1, Ordering::Relaxed);
        if idx < self.chs.len() {
            return Some(self.chs[idx].clone());
        }
        None
    }
}
