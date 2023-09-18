// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use cmake::Config as CmakeConfig;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::{
    env::{self, VarError},
    fs, io,
    path::PathBuf,
    process::Command,
};
use walkdir::WalkDir;

fn update_submodules() {
    let program = "git";
    let dir = "../";
    let args = ["submodule", "update", "--init"];
    println!(
        "Running command: \"{} {}\" in dir: {}",
        program,
        args.join(" "),
        dir
    );
    let ret = Command::new(program).current_dir(dir).args(args).status();

    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => (),
        Ok((false, Some(c))) => panic!("Command failed with error code {}", c),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Command failed with error: {}", e),
    }
}

fn erpc_include_dir() -> String {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("eRPC")
        .join("src")
        .to_str()
        .unwrap()
        .to_string()
}

fn bindgen_erpc() -> miette::Result<()> {
    println!("cargo:rerun-if-changed=src/lib.rs");
    let erpc_include_path = erpc_include_dir();
    let asio_include_path =
        PathBuf::from(erpc_include_path.clone() + "/../third_party/asio/include");
    let erpc_include_path = PathBuf::from(erpc_include_path);
    let erpc_config_include_path = PathBuf::from(env::var("OUT_DIR").unwrap())
        .join("build")
        .join("src");
    let wrapper_include_path = PathBuf::from("src");
    let mut include_path = vec![
        &asio_include_path,
        &erpc_include_path,
        &erpc_config_include_path,
        &wrapper_include_path,
    ];
    let dpdk_include_path =
        std::env::var("RTE_SDK").map(|r| PathBuf::from(r + "/build/install/usr/local/include"));
    if let Ok(dpdk_include_path) = dpdk_include_path.as_ref() {
        include_path.push(dpdk_include_path);
    }
    let mut b = autocxx_build::Builder::new("src/lib.rs", include_path).build()?;
    b.flag_if_supported("--std=c++14")
        .flag_if_supported("-Wno-unused-function")
        .compile("autocxx-erpc");
    Ok(())
}

/// If cache is stale, remove it to avoid compilation failure.
fn clean_up_stale_cache(cxx_compiler: String) {
    // We don't know the cmake output path before it's configured.
    let build_dir = format!("{}/build", env::var("OUT_DIR").unwrap());
    let path = format!("{build_dir}/CMakeCache.txt");
    let f = match std::fs::File::open(path) {
        Ok(f) => BufReader::new(f),
        // It may be an empty directory.
        Err(_) => return,
    };
    let cache_stale = f.lines().any(|l| {
        let l = l.unwrap();
        trim_start(&l, "CMAKE_CXX_COMPILER:").map_or(false, |s| {
            let mut splits = s.splitn(2, '=');
            splits.next();
            splits.next().map_or(false, |p| p != cxx_compiler)
        })
    });
    // CMake can't handle compiler change well, it will invalidate cache without respecting command
    // line settings and result in configuration failure.
    // See https://gitlab.kitware.com/cmake/cmake/-/issues/18959.
    if cache_stale {
        let _ = fs::remove_dir_all(&build_dir);
    }
}

fn is_directory_empty<P: AsRef<Path>>(p: P) -> Result<bool, io::Error> {
    let mut entries = fs::read_dir(p)?;
    Ok(entries.next().is_none())
}

fn trim_start<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.starts_with(prefix) {
        Some(s.trim_start_matches(prefix))
    } else {
        None
    }
}

fn get_env(name: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={name}");
    match env::var(name) {
        Ok(s) => Some(s),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(s)) => {
            panic!("unrecognize env var of {name}: {:?}", s.to_string_lossy());
        }
    }
}

fn prepare_erpc() {
    let modules = vec!["eRPC"];

    for module in modules {
        match is_directory_empty(module) {
            Ok(is_empty) => {
                if is_empty {
                    if module == "eRPC" {
                        update_submodules();
                    } else {
                        panic!(
                            "Can't find module {}. You need to run `git submodule \
                             update --init --recursive` first to build the project.",
                            module
                        );
                    }
                }
            }
            Err(_) => {
                panic!(
                    "Can't find module {}. You need to run `git submodule \
                     update --init --recursive` first to build the project.",
                    module
                );
            }
        }
    }
}

fn build_erpc(cc: &mut cc::Build) {
    prepare_erpc();

    let dst = {
        let mut config = CmakeConfig::new("eRPC");
        config.define("PERF", "ON");
        config.define("TRANSPORT", "dpdk");

        let cxx_compiler = if let Some(val) = get_env("CXX") {
            config.define("CMAKE_CXX_COMPILER", val.clone());
            val
        } else {
            format!("{}", cc.get_compiler().path().display())
        };
        clean_up_stale_cache(cxx_compiler);
        config.uses_cxx11().build()
    };

    let build_dir = format!("{}/build", dst.display());
    for e in WalkDir::new(&build_dir) {
        let e = e.unwrap();
        if e.file_name().to_string_lossy().ends_with(".a") {
            println!(
                "cargo:rustc-link-search=native={}",
                e.path().parent().unwrap().display()
            );
        }
    }
    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("eRPC")
            .join("build")
            .to_str()
            .unwrap()
    );

    let libs = &["erpc"];
    for l in libs {
        println!("cargo:rustc-link-lib=static={}", l);
    }
    cc.include("eRPC/src");
}

fn main() -> miette::Result<()> {
    println!("cargo:rerun-if-changed=src/erpc_wrapper.h");
    println!("cargo:rerun-if-changed=eRPC");

    let mut cc = cc::Build::new();

    build_erpc(&mut cc);

    bindgen_erpc()?;

    Ok(())
}
