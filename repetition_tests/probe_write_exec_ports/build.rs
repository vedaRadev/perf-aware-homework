use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/writes_asm.o").as_str(),
            "src/writes.asm"
        ])
        .output()
        .expect("Failed to assemble writes.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/writes_asm.o").as_str())
        .output()
        .expect("Failed to create writes_asm.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=writes_asm");
}
