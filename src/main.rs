use std::{
    env,
    process,
    io::{ prelude::*, BufReader },
    fs::File,
};

mod decoder;
use decoder::decode_instruction;

// TODO:
// Add command line option for printing disassembly (default is execute/simulate)
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Please provide an assembled 8086 instruction stream");
        process::exit(1);
    }

    let file = File::open(&args[1]).unwrap_or_else(|_| panic!("Failed to open file {}", args[1]));
    let mut instruction_stream = BufReader::new(file);

    println!("bits 16\n"); // header needed to specify 16-bit wide registers
    while !instruction_stream.fill_buf().expect("Failed to read instruction stream").is_empty() {
        if let Some(instruction) = decode_instruction(&mut instruction_stream) {
            println!("{}", instruction);
        } else {
            println!("unrecognized instruction");
        }
    }
}
