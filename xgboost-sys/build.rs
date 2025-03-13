extern crate bindgen;
extern crate cmake;

use cmake::Config;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let xgb_root = Path::new(&out_dir).join("xgboost");

    // copy source code into OUT_DIR for compilation if it doesn't exist
    if !xgb_root.exists() {
        Command::new("cp")
            .args(&["-r", "xgboost", xgb_root.to_str().unwrap()])
            .status()
            .unwrap_or_else(|e| {
                panic!("Failed to copy ./xgboost to {}: {}", xgb_root.display(), e);
            });
    }

    let mut dst = Config::new(&xgb_root);
    dst.define("BUILD_STATIC_LIB", "ON").define("CMAKE_CXX_STANDARD", "17");

    // CMake
    let mut dst = Config::new(&xgb_root);
    let mut dst = dst.define("BUILD_STATIC_LIB", "ON");

    #[cfg(feature = "cuda")]
    let mut dst = dst
        .define("USE_CUDA", "ON")
        .define("BUILD_WITH_CUDA", "ON")
        .define("BUILD_WITH_CUDA_CUB", "ON");

    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from("/opt/homebrew/"); // check for m1 vs intel config
        if let Ok(_dir) = std::fs::read_dir(&path) {
            dst = dst
                .define("CMAKE_C_COMPILER", "/opt/homebrew/opt/llvm/bin/clang")
                .define("CMAKE_CXX_COMPILER", "/opt/homebrew/opt/llvm/bin/clang++")
                .define("OPENMP_LIBRARIES", "/opt/homebrew/opt/llvm/lib")
                .define("OPENMP_INCLUDES", "/opt/homebrew/opt/llvm/include");
        };
    }
    let dst = dst.build();

    let xgb_root = xgb_root.canonicalize().unwrap();

    let bindings = bindgen::Builder::default()
        .rust_target(bindgen::RustTarget::Stable_1_77)
        .header("wrapper.h")
        .blocklist_item("std::__1.*")
        .clang_args(&["-x", "c++", "-std=c++17"])
        .clang_arg(format!("-I{}", xgb_root.join("include").display()))
        .clang_arg(format!("-I{}", xgb_root.join("rabit/include").display()))
        .clang_arg(format!("-I{}", xgb_root.join("dmlc-core/include").display()));

    #[cfg(feature = "cuda")]
    let bindings = bindings.clang_arg("-I/usr/local/cuda/include");
    let bindings = bindings
        .generate()
        .expect("Unable to generate bindings.");

    let out_path = PathBuf::from(out_dir);
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.");

    println!("cargo:rustc-link-search={}", xgb_root.join("lib").display());
    println!("cargo:rustc-link-search={}", xgb_root.join("lib64").display());
    println!("cargo:rustc-link-search={}", xgb_root.join("rabit/lib").display());
    println!("cargo:rustc-link-search={}", xgb_root.join("dmlc-core").display());

    // link to appropriate C++ lib
    if target.contains("apple") {
        println!("cargo:rustc-link-lib=c++");
        println!("cargo:rustc-link-search=native=/opt/homebrew/opt/libomp/lib");
        println!("cargo:rustc-link-lib=dylib=omp");
    } else {
        println!("cargo:rustc-cxxflags=-std=c++17");
        println!("cargo:rustc-link-lib=stdc++fs");
        println!("cargo:rustc-link-lib=stdc++");
        println!("cargo:rustc-link-lib=dylib=gomp");
    }

    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-search=native={}", dst.join("lib64").display());
    println!("cargo:rustc-link-lib=static=dmlc");
    println!("cargo:rustc-link-lib=static=xgboost");

    #[cfg(feature = "cuda")]
    {
        println!("cargo:rustc-link-search={}", "/usr/local/cuda/lib64");
        println!("cargo:rustc-link-lib=static=cudart_static");
    }
}
