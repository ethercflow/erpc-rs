// Copyright (c) 2023, IOMesh Inc. All rights reserved.

extern crate erpc_build;

use erpc_build::codegen;

fn main() {
    codegen::protoc_gen_erpc_rust_main();
}
