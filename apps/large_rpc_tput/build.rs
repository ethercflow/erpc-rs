extern crate pkg_config;
extern crate regex;

use regex::Regex;
use std::env;
use std::path::Path;
use std::process::exit;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=DPDK_PATH");
    println!("cargo:rerun-if-changed=build.rs");

    let dpdk_path = env::var("DPDK_PATH").unwrap();
    let dpdk_path = Path::new(&dpdk_path);
    // TODO: support Centos
    let dpdk_pkg_config_path = dpdk_path.join("lib/x86_64-linux-gnu/pkgconfig");
    let dpdk_ldflags = Command::new("pkg-config")
        .env("PKG_CONFIG_PATH", &dpdk_pkg_config_path)
        .args(["--static", "--libs-only-l", "libdpdk"])
        .output()
        .unwrap_or_else(|e| panic!("Failed pkg-config ldflags: {:?}", e))
        .stdout;
    if dpdk_ldflags.is_empty() {
        eprintln!("Could not get DPDK's LDFLAGS. Is DPDK_PATH set correctly?");
        exit(1);
    }
    let dpdk_ldflags = String::from_utf8(dpdk_ldflags).unwrap();
    let mut dpdk_link_names = vec![];
    let lib_name_format = Regex::new(r"lib(.*)\.(a)").unwrap();
    for ldflag in dpdk_ldflags.split(' ') {
        if let Some(lib_name) = ldflag.strip_prefix("-l") {
            if let Some(capture) = lib_name_format.captures(lib_name) {
                let link_name = &capture[1];
                dpdk_link_names.push(link_name.to_string());
            }
        }
    }
    println!(
        "cargo:rustc-link-search=native={}",
        dpdk_pkg_config_path.parent().unwrap().to_str().unwrap()
    );
    for lib_name in dpdk_link_names {
        println!(
            "cargo:rustc-link-lib=static:+whole-archive,-bundle={}",
            lib_name
        );
    }
    [
        "z", "jansson", "bsd", "atomic", "numa", "ibverbs", "mlx4", "mlx5",
    ]
    .map(|lib| {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    });
}
