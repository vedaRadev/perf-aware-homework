use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/load_alignment_offset.o").as_str(),
            "src/load_alignment_offset.asm"
        ])
        .output()
        .expect("Failed to assemble load_alignment_offset.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/load_alignment_offset.o").as_str())
        .output()
        .expect("Failed to create load_alignment_offset.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=load_alignment_offset");
}
