use erpc_build::codegen::Generator;
use prost_build::Config;
use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=proto/largerpctput.proto");
    Config::new()
        .service_generator(Box::new(Generator))
        .out_dir("src")
        .compile_protos(&["proto/largerpctput.proto"], &["proto"])?;
    Ok(())
}
