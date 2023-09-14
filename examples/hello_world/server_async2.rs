// Copyright (c) 2023, IOMesh Inc. All rights reserved.

mod common;

use anyhow::Result;
use common::*;
use crossbeam::channel;
use crossbeam_channel::{unbounded, TryRecvError};
use erpc_rs::prelude::*;
use std::{
    mem::{self, MaybeUninit},
    slice, thread,
};
use tokio::{fs::File, io::AsyncReadExt, runtime::Handle, sync::mpsc};

extern "C" fn req_handler(req_handle: *mut RawReqHandle, context: *mut c_void) {
    let c = unsafe { &mut *(context as *mut AppContext) };
    let req_handle = ReqHandle::from_inner_raw(req_handle);
    c.rt.spawn(async {
        c.tx.send(Req {
            req_handle,
            rpc: unsafe { c.rpc.assume_init_mut() },
            file_name: "./hello.txt".to_string(),
        })
        .unwrap();
    });
}

struct Req<'a> {
    pub req_handle: ReqHandle,
    pub rpc: &'a mut Rpc,
    pub file_name: String,
}

struct Resp {
    pub req_handle: ReqHandle,
    pub msg_buf: MsgBuffer,
}

struct AppContext<'a> {
    pub rpc: MaybeUninit<Rpc>,
    // used to send req to tokio
    pub tx: mpsc::UnboundedSender<Req<'a>>,
    // used to recv resp from tokio
    pub rx: channel::Receiver<Resp>,
    pub rt: Handle,
}

impl Drop for AppContext<'_> {
    fn drop(&mut self) {
        unsafe {
            self.rpc.assume_init_drop();
        }
    }
}

unsafe impl Send for AppContext<'_> {}
unsafe impl Sync for AppContext<'_> {}

#[tokio::main]
async fn main() -> Result<()> {
    let server_uri = K_SERVER_HOST_NAME.to_owned() + ":" + K_UDP_PORT;

    let (tx, mut rx) = mpsc::unbounded_channel::<Req>();
    let (tx1, rx1) = unbounded::<Resp>();
    let rt = Handle::current();

    thread::spawn(move || {
        let mut c = AppContext {
            rpc: MaybeUninit::uninit(),
            tx,
            rx: rx1,
            rt,
        };
        let mut nexus = Nexus::new(&server_uri, 0);
        nexus.register_req_func(K_REQ_TYPE, req_handler).unwrap();
        let rpc = Rpc::new(
            &mut nexus,
            Some(&mut c as *mut AppContext as *mut c_void),
            0,
            None,
            0,
        );

        unsafe {
            c.rpc.as_mut_ptr().write(rpc);
        }

        loop {
            match c.rx.try_recv() {
                Ok(mut resp) => unsafe {
                    let mut resp_msgbuf = resp.req_handle.get_dyn_resp_msgbuf();
                    mem::swap(&mut resp_msgbuf, &mut resp.msg_buf);
                    c.rpc
                        .assume_init_mut()
                        .enqueue_response(&mut resp.req_handle, &mut resp_msgbuf);
                    mem::swap(&mut resp.msg_buf, &mut resp_msgbuf);
                    c.rpc.assume_init_mut().free_msg_buffer(&resp.msg_buf);
                },
                Err(TryRecvError::Empty) => {}
                _ => unreachable!(),
            }
            unsafe {
                c.rpc.assume_init_mut().run_event_loop(0);
            }
        }
    });

    loop {
        let req = rx.recv().await.unwrap();
        let tx1 = tx1.clone();
        tokio::spawn(async move {
            let mut file = File::open(req.file_name).await.unwrap();
            let metadata = file.metadata().await.unwrap();
            let resp_size = metadata.len() as usize;
            let msg_buf = req.rpc.alloc_msg_buffer_or_die(resp_size);
            let mut msg_buf_vec =
                unsafe { slice::from_raw_parts(msg_buf.get_inner_buf(), resp_size).to_vec() };
            file.read_to_end(&mut msg_buf_vec).await.unwrap();
            tx1.send(Resp {
                req_handle: req.req_handle,
                msg_buf,
            })
            .unwrap();
        });
    }
}
