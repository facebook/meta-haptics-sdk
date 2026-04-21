// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    //
    // Run cc to compile the C renderer as a static library and link against it
    //
    let mut build = cc::Build::new();
    let renderer_c_path = Path::new("../renderer_c");
    let renderer_c_include_path = renderer_c_path.join("include");
    let renderer_c_include_path_full = renderer_c_include_path.join("haptic_renderer");
    let renderer_c_src_path = renderer_c_path.join("src");

    for header_path in fs::read_dir(&renderer_c_include_path_full).unwrap() {
        println!(
            "cargo:rerun-if-changed={}",
            header_path.unwrap().path().to_str().unwrap()
        );
    }

    for src_path in fs::read_dir(renderer_c_src_path).unwrap() {
        let src_path = src_path.as_ref().unwrap().path();
        if src_path.extension().unwrap().to_str().unwrap() == "c" {
            build.file(&src_path);
        }
        println!("cargo:rerun-if-changed={}", src_path.to_str().unwrap());
    }

    build.include(&renderer_c_include_path);
    build.compile("renderer_c");

    //
    // Run bindgen to convert renderer_h to renderer_c.rs
    //
    let bindings = bindgen::Builder::default().header(
        renderer_c_include_path_full
            .join("renderer.h")
            .to_str()
            .unwrap(),
    );
    let bindings = bindings
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap();
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("renderer_c.rs"))
        .unwrap();
}
