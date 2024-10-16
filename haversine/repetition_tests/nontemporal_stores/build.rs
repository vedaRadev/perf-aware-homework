use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/test_routines.o").as_str(),
            "src/test_routines.asm"
        ])
        .output()
        .expect("Failed to assemble test_routines.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/test_routines.o").as_str())
        .output()
        .expect("Failed to create test_routines.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=test_routines");
}
