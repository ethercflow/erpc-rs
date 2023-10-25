// Copyright (c) 2023, IOMesh Inc. All rights reserved.

#![feature(get_mut_unchecked)]

mod cli;
mod client;
mod common;
mod largerpctput;
mod server;

use cli::parse_args;
use client::client_main;
use server::server_main;

#[tokio::main]
async fn main() {
    let args = parse_args();
    if args.process_id == 0 {
        server_main(args).await.unwrap();
    } else {
        client_main(args).await.unwrap();
    }
}
