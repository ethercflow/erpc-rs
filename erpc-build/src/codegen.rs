// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{
    io::{Error, ErrorKind, Read},
    path::Path,
    sync::atomic::{AtomicU8, Ordering},
    {env, fs, io, process::Command, str},
};

use derive_new::new;
use prost::Message;
use prost_build::{Config, Method, Service, ServiceGenerator};
use prost_types::FileDescriptorSet;

use crate::util::{fq_erpc, to_snake_case};

static METHOD_ID: AtomicU8 = AtomicU8::new(1);

/// Returns the names of all packages compiled.
pub fn compile_protos<P>(protos: &[P], includes: &[P], out_dir: &str) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let mut prost_config = Config::new();
    prost_config.service_generator(Box::new(Generator));
    prost_config.out_dir(out_dir);

    // Create a file descriptor set for the protocol files.
    let tmp = tempfile::Builder::new().prefix("prost-build").tempdir()?;
    std::fs::create_dir_all(tmp.path())?;
    let descriptor_set = tmp.path().join("prost-descriptor-set");

    let mut cmd = Command::new(prost_build::protoc_from_env());
    cmd.arg("--include_imports")
        .arg("--include_source_info")
        .arg("-o")
        .arg(&descriptor_set);

    for include in includes {
        cmd.arg("-I").arg(include.as_ref());
    }

    // Set the protoc include after the user includes in case the user wants to
    // override one of the built-in .protos.
    if let Some(inc) = prost_build::protoc_include_from_env() {
        cmd.arg("-I").arg(inc);
    }

    for proto in protos {
        cmd.arg(proto.as_ref());
    }

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::new(
            ErrorKind::Other,
            format!("protoc failed: {}", String::from_utf8_lossy(&output.stderr)),
        ));
    }

    let mut buf = Vec::new();
    fs::File::open(descriptor_set)?.read_to_end(&mut buf)?;
    let descriptor_set = FileDescriptorSet::decode(buf.as_slice())?;

    // Get the package names from the descriptor set.
    let mut packages: Vec<_> = descriptor_set
        .file
        .iter()
        .filter_map(|f| f.package.clone())
        .collect();
    packages.sort();
    packages.dedup();

    // FIXME(https://github.com/danburkert/prost/pull/155)
    // Unfortunately we have to forget the above work and use `compile_protos` to
    // actually generate the Rust code.
    prost_config.compile_protos(protos, includes)?;

    Ok(packages)
}

/// [`ServiceGenerator`](prost_build::ServiceGenerator) for generating erpc services.
///
/// Can be used for when there is a need to deviate from the common use case of
/// [`compile_protos()`]. One can provide a `Generator` instance to
/// [`prost_build::Config::service_generator()`].
///
/// ```rust
/// use prost_build::Config;
/// use erpc_build::codegen::Generator;
///
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut config = Config::new();
///     config.service_generator(Box::new(Generator));
///     // Modify config as needed
///     config.compile_protos(&["src/frontend.proto", "src/backend.proto"], &["src"])?;
///     Ok(())
/// }
/// ```
pub struct Generator;

impl ServiceGenerator for Generator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        generate_methods(&service, buf);
        generate_client(&service, buf);
        generate_server(&service, buf);
    }
}

fn generate_methods(service: &Service, buf: &mut String) {
    for method in &service.methods {
        generate_method(&service.name, method, buf);
    }
}

fn const_method_name(service_name: &str, method: &Method) -> String {
    format!(
        "METHOD_{}_{}",
        to_snake_case(service_name).to_uppercase(),
        method.name.to_uppercase()
    )
}

fn generate_method(service_name: &str, method: &Method, buf: &mut String) {
    let name = const_method_name(service_name, method);
    let ty = format!(
        "{}<{}, {}>",
        fq_erpc("Method"),
        method.input_type,
        method.output_type
    );

    buf.push_str("pub const ");
    buf.push_str(&name);
    buf.push_str(": ");
    buf.push_str(&ty);
    buf.push_str(" = ");
    generate_method_body(buf);
}

fn generate_method_body(buf: &mut String) {
    let pr_mar = format!(
        "{} {{ ser: {}, de: {} }}",
        fq_erpc("Marshaller"),
        fq_erpc("pr_ser"),
        fq_erpc("pr_de")
    );
    let id = METHOD_ID.fetch_add(1, Ordering::SeqCst);

    buf.push_str(&fq_erpc("Method"));
    buf.push('{');
    generate_field_init("id", &id.to_string(), buf);
    generate_field_init("req_mar", &pr_mar, buf);
    generate_field_init("resp_mar", &pr_mar, buf);
    buf.push_str("};\n");
}

fn generate_field_init(name: &str, value: &str, buf: &mut String) {
    buf.push_str(name);
    buf.push_str(": ");
    buf.push_str(value);
    buf.push_str(", ");
}

fn generate_client(service: &Service, buf: &mut String) {
    let client_name = format!("{}Client", service.name);
    buf.push_str("#[derive(Clone)]\n");
    buf.push_str("pub struct ");
    buf.push_str(&client_name);
    buf.push_str(" { pub client: ::erpc_rs::prelude::Client }\n");

    buf.push_str("impl ");
    buf.push_str(&client_name);
    buf.push_str(" {\n");
    generate_ctor(&client_name, buf);
    generate_client_methods(service, buf);
    buf.push_str("}\n")
}

fn generate_ctor(client_name: &str, buf: &mut String) {
    buf.push_str("pub fn new(channel: ::erpc_rs::prelude::Channel) -> Self { ");
    buf.push_str(client_name);
    buf.push_str(" { client: ::erpc_rs::prelude::Client::new(channel) }");
    buf.push_str("}\n");
}

fn generate_client_methods(service: &Service, buf: &mut String) {
    for method in &service.methods {
        generate_client_method(&service.name, method, buf);
    }
    generate_client_alloc_msg_buffer(buf)
}

fn generate_client_method(service_name: &str, method: &Method, buf: &mut String) {
    let name = &format!(
        "METHOD_{}_{}",
        to_snake_case(service_name).to_uppercase(),
        method.name.to_uppercase()
    );
    ClientMethod::new(
        &method.name,
        Some(&method.input_type),
        vec![&method.output_type],
        "unary_call",
        name,
    )
    .generate(buf);
}

#[derive(new)]
struct ClientMethod<'a> {
    method_name: &'a str,
    request: Option<&'a str>,
    result_types: Vec<&'a str>,
    inner_method_name: &'a str,
    data_name: &'a str,
}

impl<'a> ClientMethod<'a> {
    fn generate(&self, buf: &mut String) {
        buf.push_str("pub async fn ");

        buf.push_str(self.method_name);

        buf.push_str("(&self");
        if let Some(req) = self.request {
            buf.push_str(", req: &");
            buf.push_str(req);
        }
        buf.push_str(", req_msgbuf: std::sync::Arc<");
        buf.push_str(&fq_erpc("MsgBuffer"));
        buf.push_str(">, resp_msgbuf: std::sync::Arc<");
        buf.push_str(&fq_erpc("MsgBuffer"));
        buf.push_str(">, cb: ");
        buf.push_str(&fq_erpc("ContFunc"));
        buf.push_str(") -> ");

        buf.push_str(&fq_erpc("Result"));
        buf.push('<');
        if self.result_types.len() != 1 {
            buf.push('(');
        }
        for rt in &self.result_types {
            buf.push_str(rt);
            buf.push(',');
        }
        if self.result_types.len() != 1 {
            buf.push(')');
        }
        buf.push_str("> { ");
        self.generate_inner_body(buf);
        buf.push_str(" }\n");
    }

    // Method delegates to the inner client.
    fn generate_inner_body(&self, buf: &mut String) {
        buf.push_str("self.client.");
        buf.push_str(self.inner_method_name);
        buf.push_str("(&");
        buf.push_str(self.data_name);
        if self.request.is_some() {
            buf.push_str(", req, req_msgbuf, resp_msgbuf, cb");
        }
        buf.push_str(").await");
    }
}

fn generate_client_alloc_msg_buffer(buf: &mut String) {
    buf.push_str("pub fn alloc_msg_buffer(&mut self, max_data_size: usize) -> ");
    buf.push_str(&fq_erpc("MsgBuffer"));
    buf.push_str("{\n");
    buf.push_str("self.client.alloc_msg_buffer(max_data_size)\n");
    buf.push_str("}\n");
}

fn generate_server(service: &Service, buf: &mut String) {
    buf.push_str("#[async_trait::async_trait]\n");
    buf.push_str("pub trait ");
    buf.push_str(&service.name);
    buf.push_str(": Send + 'static {\n");
    generate_server_methods(service, buf);
    buf.push_str("}\n");
    generate_server_method_wrappers(service, buf);

    buf.push_str("pub fn create_");
    buf.push_str(&to_snake_case(&service.name));
    buf.push_str("<S: ");
    buf.push_str(&service.name);
    buf.push_str(" + Send + Clone + 'static>() -> ");
    buf.push_str(&fq_erpc("Service"));
    buf.push_str(" {\n");
    buf.push_str("let mut builder = ::erpc_rs::prelude::ServiceBuilder::new();\n");

    for method in &service.methods {
        generate_method_bind(&service.name, method, buf);
    }

    buf.push_str("builder.build()\n");
    buf.push_str("}\n");
}

fn generate_server_methods(service: &Service, buf: &mut String) {
    for method in &service.methods {
        generate_server_method(method, buf, true);
        generate_server_method(method, buf, false);
    }
}

fn generate_server_method(method: &Method, buf: &mut String, sync: bool) {
    if !sync {
        buf.push_str("async ");
    }
    buf.push_str("fn ");
    buf.push_str(&method.name);
    if !sync {
        buf.push_str("_async");
    }
    buf.push_str("(_req: ");
    buf.push_str(&fq_erpc("ReqHandle"));
    if sync {
        buf.push_str(", _ctx: &'static mut ");
        buf.push_str(&fq_erpc("RpcContext"));
    }
    if !sync {
        buf.push_str(", _codec: ");
        buf.push_str(&fq_erpc("Codec"));
        let ty = format!("<{}, {}>", method.input_type, method.output_type);
        buf.push_str(&ty);
    }
    buf.push_str(") { unimplemented!() }\n");
}

fn generate_server_method_wrappers(service: &Service, buf: &mut String) {
    for method in &service.methods {
        generate_server_method_wrapper(&service.name, method, buf);
    }
}

fn generate_server_method_wrapper(srv_name: &str, method: &Method, buf: &mut String) {
    buf.push_str("extern \"C\" fn ");
    buf.push_str(&method.name);
    buf.push_str("_wrapper");
    buf.push_str("<S: ");
    buf.push_str(srv_name);
    buf.push_str(">(req: *mut");
    buf.push_str(&fq_erpc("RawReqHandle"));
    buf.push_str(", ctx: *mut erpc_rs::prelude::c_void) {\n");
    generate_wrapper_inner_body(method, buf);
    buf.push_str("}\n");
    buf.push_str("unsafe fn ");
    buf.push_str(&method.name);
    buf.push_str("_wrapper_into");
    buf.push_str("<S: ");
    buf.push_str(srv_name);
    buf.push_str(">() -> ");
    buf.push_str(&fq_erpc("ReqHandler"));
    buf.push_str(" {\n");
    buf.push_str(&method.name);
    buf.push_str("_wrapper::<S>\n");
    buf.push_str("}\n");
}

fn generate_wrapper_inner_body(method: &Method, buf: &mut String) {
    buf.push_str("let result = std::panic::catch_unwind(|| {\n");
    buf.push_str("let req = erpc_rs::prelude::ReqHandle::from_inner_raw(req);\n");
    buf.push_str("let ctx = unsafe { &mut *(ctx as *mut ::erpc_rs::prelude::RpcContext) };\n");
    buf.push_str("S::");
    buf.push_str(&method.name);
    buf.push_str("(req, ctx);\n");
    buf.push_str("});\n");
    buf.push_str("if result.is_err() {\n");
    buf.push_str("std::process::abort();");
    buf.push_str("}\n");
}

fn generate_method_bind(service_name: &str, method: &Method, buf: &mut String) {
    let add_name = "add_unary_handler";

    buf.push_str("builder = builder.");
    buf.push_str(add_name);
    buf.push_str("(&");
    buf.push_str(&const_method_name(service_name, method));
    buf.push_str(", move |req, codec| { std::boxed::Box::pin(S::");
    buf.push_str(&method.name);
    buf.push_str("_async(req, codec))}");
    buf.push_str(", unsafe {");
    buf.push_str(&method.name);
    buf.push_str("_wrapper_into::<S>() }, \n");
    buf.push_str(");\n");
}

pub fn protoc_gen_erpc_rust_main() {
    let mut args = env::args();
    args.next();
    let (mut protos, mut includes, mut out_dir): (Vec<_>, Vec<_>, _) = Default::default();
    for arg in args {
        if let Some(value) = arg.strip_prefix("--protos=") {
            eprintln!("value: {}", value);
            protos.extend(value.split(",").map(|s| s.to_string()));
        } else if let Some(value) = arg.strip_prefix("--includes=") {
            includes.extend(value.split(",").map(|s| s.to_string()));
        } else if let Some(value) = arg.strip_prefix("--out-dir=") {
            out_dir = value.to_string();
        }
    }
    if protos.is_empty() {
        panic!("should at least specify protos to generate");
    }
    compile_protos(&protos, &includes, &out_dir).unwrap();
}
