use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    // Assemble and create lib for write_all_bytes.asm
    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/write_all_bytes_asm.o").as_str(),
            "asm/write_all_bytes.asm"
        ])
        .output()
        .expect("Failed to assemble write_all_bytes.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/write_all_bytes_asm.o").as_str())
        .output()
        .expect("Failed to create write_all_bytes.lib");


    // Assemble and create lib for nop_loops.asm
    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/nop_loops_asm.o").as_str(),
            "asm/nop_loops.asm"
        ])
        .output()
        .expect("Failed to assemble nop_loops.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/nop_loops_asm.o").as_str())
        .output()
        .expect("Failed to create nop_loops_asm.lib");

    // Link libs to program
    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=write_all_bytes_asm");
    println!("cargo:rustc-link-lib=static=nop_loops_asm");
}
