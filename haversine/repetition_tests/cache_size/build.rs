use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("No OUT_DIR environment variable provided");

    Command::new("nasm")
        .args([
            "-f", "win64",
            "-o", format!("{out_dir}/cache_tests.o").as_str(),
            "src/cache_tests.asm"
        ])
        .output()
        .expect("Failed to assemble cache_tests.asm");

    Command::new("lib")
        .arg(format!("{out_dir}/cache_tests.o").as_str())
        .output()
        .expect("Failed to create cache_tests.lib");

    println!("cargo:rustc-link-search=native={out_dir}");
    println!("cargo:rustc-link-lib=static=cache_tests");
}
