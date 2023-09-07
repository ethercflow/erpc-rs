use std::path::Path;
use std::{env, fs, path::PathBuf, process::Command};

fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!("The `{name}` directory is empty, did you forget to pull the submodules?");
        println!("Try `git submodule update --init --recursive`");
        panic!();
    }
}
fn try_to_find_and_link_lib(lib_name: &str) -> bool {
    println!("cargo:rerun-if-env-changed={lib_name}_COMPILE");
    if let Ok(v) = env::var(format!("{lib_name}_COMPILE")) {
        if v.to_lowercase() == "true" || v == "1" {
            return false;
        }
    }

    println!("cargo:rerun-if-env-changed={lib_name}_LIB_DIR");
    println!("cargo:rerun-if-env-changed={lib_name}_STATIC");

    if let Ok(lib_dir) = env::var(format!("{lib_name}_LIB_DIR")) {
        println!("cargo:rustc-link-search=native={lib_dir}");
        let mode = match env::var_os(format!("{lib_name}_STATIC")) {
            Some(_) => "static",
            None => "dylib",
        };
        println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());
        return true;
    }
    false
}

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
    match env::var("ERPC_INCLUDE_DIR") {
        Ok(val) => val,
        Err(_) => "./erpc/src".to_string(),
    }
}

fn bindgen_erpc() -> miette::Result<()> {
    let asio_include_path = PathBuf::from(erpc_include_dir() + "/../third_party/asio/include");
    let erpc_include_path = PathBuf::from(erpc_include_dir());
    let wrapper_include_path = PathBuf::from("src");
    let mut include_path = vec![
        &asio_include_path,
        &erpc_include_path,
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
    println!("cargo:rerun-if-changed=src/lib.rs");
    Ok(())
}

fn build_erpc() {
    let dir = "./erpc";
    let mut program = "cmake";
    let mut args = ["-DPERF=OFF", "-DTRANSPORT=dpdk"];
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

    program = "make";
    args = ["-j", "4"];
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

fn main() -> miette::Result<()> {
    if !Path::new("erpc/README.md").exists() {
        update_submodules();
    }
    if !try_to_find_and_link_lib("ERPC") {
        println!("cargo:rerun-if-changed=erpc/");
        fail_on_empty_directory("erpc");
        build_erpc();
    }
    bindgen_erpc()?;

    Ok(())
}
