// Copyright (c) 2023, IOMesh Inc. All rights reserved.

extern crate regex;

use cmake::Config as CmakeConfig;
use pkg_config::Config as PkgConfig;
use regex::Regex;
use std::collections::HashSet;
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
    let args = ["submodule", "update", "--init", "--recursive"];
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

fn bindgen_erpc(use_sys_dpdk: bool) -> miette::Result<()> {
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
    let dpdk_include_path = PathBuf::from(if use_sys_dpdk {
        get_env("RTE_INCLUDE_PATH").unwrap()
    } else {
        format!(
            "{}/dpdk/build/install/usr/local/include",
            env::var("OUT_DIR").unwrap()
        )
    });
    include_path.push(&dpdk_include_path);
    let mut b = autocxx_build::Builder::new("src/lib.rs", include_path).build()?;
    b.flag_if_supported("--std=c++14")
        .flag_if_supported("-Wno-unused-function")
        .static_flag(true)
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

fn prepare_module(module: &str) {
    match is_directory_empty(module) {
        Ok(is_empty) => {
            if is_empty {
                update_submodules();
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

fn build_rdma(cc: &mut cc::Build) {
    prepare_module("rdma-core");

    let dst = {
        let mut config = CmakeConfig::new("rdma-core");
        config.define("ENABLE_STATIC", "ON");
        config.define("NO_PYVERBS", "1");
        config.define("ENABLE_RESOLVE_NEIGH", "0");
        config.define("CMAKE_BUILD_TYPE", "Release");
        config.define(
            "CMAKE_INSTALL_PREFIX",
            format!("{}/r", env::var("OUT_DIR").unwrap()),
        );

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
    let lib_dir = build_dir + "/lib";
    for e in WalkDir::new(&lib_dir) {
        let e = e.unwrap();
        if e.file_name().to_string_lossy().ends_with(".a") {
            println!(
                "cargo:rustc-link-search=native={}",
                e.path().parent().unwrap().display()
            );
        }
    }
    cc.include("build_dir/include");
}

fn build_dpdk() {
    prepare_module("dpdk");

    let build_dir = format!("{}/build", env::var("OUT_DIR").unwrap());
    let c_include_path = build_dir.clone() + "/../r/include";
    let library_path = build_dir.clone() + "/../r/lib";
    if fs::metadata("./dpdk/build/build.ninja").is_err() {
        let program = "meson";
        let args = [
            "-Dlibdir=lib",
            "-Dincludedir=include",
            "-Dexamples=",
            "-Denable_kmods=false",
            "-Dtests=false",
            "-Ddisable_drivers=raw/*,crypto/*,baseband/*,dma/*",
            "build",
        ];
        let ret = Command::new(program)
            .env("C_INCLUDE_PATH", &c_include_path)
            .env("LIBRARY_PATH", &library_path)
            .current_dir("./dpdk")
            .args(args)
            .status();
        match ret.map(|status| (status.success(), status.code())) {
            Ok((true, _)) => (),
            Ok((false, Some(c))) => panic!("Command failed with error code {}", c),
            Ok((false, None)) => panic!("Command got killed"),
            Err(e) => panic!("Command failed with error: {}", e),
        }
    }

    let program = "ninja";
    let args = ["install"];
    let ret = Command::new(program)
        .env("C_INCLUDE_PATH", &c_include_path)
        .env("LIBRARY_PATH", &library_path)
        .env("DESTDIR", build_dir + "/../dpdk/build/install")
        .current_dir("./dpdk/build")
        .args(args)
        .status();
    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => (),
        Ok((false, Some(c))) => panic!("Command failed with error code {}", c),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Command failed with error: {}", e),
    }
}

fn link_with_rdma(dst: &Path, use_sys: bool) {
    let path = if use_sys {
        get_env("RDMA_PKG_CONFIG").unwrap()
    } else {
        let mut path = format!("{}/r/lib/pkgconfig", dst.display());
        if is_directory_empty(&path).is_err() {
            path = format!("{}/r/lib64/pkgconfig", dst.display());
            if is_directory_empty(&path).is_err() || is_directory_empty(&path).unwrap() {
                panic!("r's pkgconfig path {} not correct", path);
            }
        }
        path
    };
    env::set_var("PKG_CONFIG_PATH", &path);
    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(path).parent().unwrap().display()
    );
    let mut cfg = PkgConfig::new();
    cfg.print_system_cflags(false)
        .print_system_libs(false)
        .env_metadata(false)
        .cargo_metadata(false)
        .statik(true);
    let rdma = cfg.probe("libibverbs").unwrap();
    let rdma_libs: HashSet<_> = rdma.libs.iter().cloned().collect();
    for l in rdma_libs.iter() {
        if l != "pthread" {
            println!("cargo:rustc-link-lib=static:+whole-archive,-bundle={l}");
        }
    }
}

fn link_with_dpdk(dst: &Path, use_sys: bool) {
    let path = if use_sys {
        get_env("RTE_PKG_CONFIG").unwrap()
    } else {
        let path = format!(
            "{}/dpdk/build/install/usr/local/lib/pkgconfig",
            dst.display()
        );
        if is_directory_empty(&path).is_err() || is_directory_empty(&path).unwrap() {
            panic!("dpdk's pkgconfig path {} not correct", path);
        }
        path
    };
    env::set_var("PKG_CONFIG_PATH", &path);
    println!(
        "cargo:rustc-link-search=native={}",
        PathBuf::from(path).parent().unwrap().display()
    );
    let mut cfg = PkgConfig::new();
    cfg.print_system_cflags(false)
        .print_system_libs(false)
        .env_metadata(false)
        .cargo_metadata(false)
        .statik(true);
    let dpdk = cfg.probe("libdpdk").unwrap();
    let dpdk_libs: HashSet<_> = dpdk.libs.iter().cloned().collect();
    let lib_name_format = Regex::new(r"lib(.*)\.(a)").unwrap();
    let mut dpdk_link_names = vec![];
    for l in dpdk_libs.iter() {
        if let Some(l) = l.strip_prefix(':') {
            if let Some(capture) = lib_name_format.captures(l) {
                let link_name = &capture[1];
                dpdk_link_names.push(link_name.to_string());
            }
        }
    }
    for l in dpdk_link_names {
        println!("cargo:rustc-link-lib=static:+whole-archive,-bundle={l}");
    }
}

fn build_erpc(cc: &mut cc::Build, use_sys_rdma: bool, use_sys_dpdk: bool) {
    prepare_module("eRPC");

    let dst = {
        let mut config = CmakeConfig::new("eRPC");
        config.define("PERF", "ON");
        config.define("TRANSPORT", "dpdk");
        config.env(
            "RTE_SDK",
            if use_sys_dpdk {
                get_env("RTE_SDK").unwrap()
            } else {
                format!("{}/dpdk", env::var("OUT_DIR").unwrap())
            },
        );

        let cxx_compiler = if let Some(val) = get_env("CXX") {
            config.define("CMAKE_CXX_COMPILER", val.clone());
            val
        } else {
            format!("{}", cc.get_compiler().path().display())
        };
        clean_up_stale_cache(cxx_compiler);

        if use_sys_rdma {
            config.env("LIBRARY_PATH", get_env("RDMA_PKG_CONFIG").unwrap() + "/../");
        } else {
            let out_dir = env::var("OUT_DIR").unwrap().to_string();
            let mut path = out_dir.clone() + "/r/lib";
            if is_directory_empty(&path).is_err() {
                path = out_dir.clone() + "/r/lib64";
                if is_directory_empty(&path).is_err() || is_directory_empty(&path).unwrap() {
                    panic!("rdma core's lib path {} not correct", path);
                }
            }
            config.env("LIBRARY_PATH", path);
        }

        config.uses_cxx11().build()
    };

    let rdma_lib_dir = if use_sys_rdma {
        get_env("RDMA_PKG_CONFIG").unwrap() + "/../"
    } else {
        format!("{}/build", dst.display())
    };
    for e in WalkDir::new(&rdma_lib_dir) {
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

    link_with_rdma(&dst, use_sys_rdma);
    link_with_dpdk(&dst, use_sys_dpdk);

    let libs = &["erpc"];
    for l in libs {
        println!("cargo:rustc-link-lib=static:+whole-archive,-bundle={}", l);
    }
    ["z", "jansson", "bsd", "numa", "pthread", "archive"].map(|lib| {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    });
    if let Ok(os_release) = std::fs::read_to_string("/etc/os-release") {
        if os_release.contains("Ubuntu") {
            println!("cargo:rustc-link-lib=dylib=atomic");
        }
    }

    cc.include("eRPC/src");
}

fn main() -> miette::Result<()> {
    println!("cargo:rerun-if-changed=src/erpc_wrapper.h");
    println!("cargo:rerun-if-changed=eRPC");

    let mut use_sys_rdma = false;
    let mut use_sys_dpdk = false;
    let mut cc = cc::Build::new();

    if get_env("RDMA_PKG_CONFIG").is_none() {
        build_rdma(&mut cc);
    } else {
        use_sys_rdma = true;
    }
    if get_env("RTE_SDK").is_none() {
        build_dpdk();
    } else {
        use_sys_dpdk = true;
    }
    build_erpc(&mut cc, use_sys_rdma, use_sys_dpdk);

    bindgen_erpc(use_sys_dpdk)?;

    Ok(())
}
