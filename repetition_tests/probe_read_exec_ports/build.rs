use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/reads_asm.o").as_str(),
            "src/reads.asm"
        ])
        .output()
        .expect("Failed to assemble reads.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/reads_asm.o").as_str())
        .output()
        .expect("Failed to create reads_asm.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=reads_asm");
}
