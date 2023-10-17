use erpc_build::codegen::Generator;
use prost_build::Config;
use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=proto/helloworld.proto");
    let mut config = Config::new();
    config.service_generator(Box::new(Generator));
    config.compile_protos(&["proto/helloworld.proto"], &["proto"])?;
    Ok(())
}
