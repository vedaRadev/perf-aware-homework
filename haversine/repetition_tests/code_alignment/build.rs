use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/code_alignment_asm.o").as_str(),
            "src/code_alignment.asm"
        ])
        .output()
        .expect("Failed to assemble code_alignment.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/code_alignment_asm.o").as_str())
        .output()
        .expect("Failed to create code_alignment_asm.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=code_alignment_asm");
}
