use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/read_widths_asm.o").as_str(),
            "src/read_widths.asm"
        ])
        .output()
        .expect("Failed to assemble read_widths.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/read_widths_asm.o").as_str())
        .output()
        .expect("Failed to create read_widths_asm.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=read_widths_asm");
}
