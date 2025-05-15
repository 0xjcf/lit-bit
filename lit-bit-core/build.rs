// build.rs
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    if target.starts_with("riscv32") {
        let source_path = "memory_riscv_source.x";
        println!("cargo:rerun-if-changed={source_path}");
        if fs::metadata(source_path).is_ok() {
            let content = include_bytes!("memory_riscv_source.x");
            fs::write(out_dir.join("memory.x"), content).unwrap();
        } else {
            panic!("Error: Source script {source_path} not found for target {target}");
        }
    } else if target.starts_with("thumbv7m") {
        let source_path = "memory_cortex_m_source.x";
        println!("cargo:rerun-if-changed={source_path}");
        if fs::metadata(source_path).is_ok() {
            let content = include_bytes!("memory_cortex_m_source.x");
            fs::write(out_dir.join("memory.x"), content).unwrap();
        } else {
            panic!("Error: Source script {source_path} not found for target {target}");
        }
    }

    println!("cargo:rustc-link-search={}", out_dir.display());
}
