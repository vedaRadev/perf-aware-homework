use std::{
    env,
    process,
    fs,
    io::{ prelude::*, BufReader },
};

//      FULL 16 BIT REG                 LOWER 8 BIT REG                 UPPER 8 BIT REG
const REG_NAME_AX: &str = "ax"; const REG_NAME_AL: &str = "al"; const REG_NAME_AH: &str = "ah";
const REG_NAME_CX: &str = "cx"; const REG_NAME_CL: &str = "cl"; const REG_NAME_CH: &str = "ch";
const REG_NAME_DX: &str = "dx"; const REG_NAME_DL: &str = "dl"; const REG_NAME_DH: &str = "dh";
const REG_NAME_BX: &str = "bx"; const REG_NAME_BL: &str = "bl"; const REG_NAME_BH: &str = "bh";
const REG_NAME_SP: &str = "sp";
const REG_NAME_BP: &str = "bp";
const REG_NAME_SI: &str = "si";
const REG_NAME_DI: &str = "di";

const OP_NAME_MOV: &str = "mov";

fn get_register_name(encoded_register: u8, wide: bool) -> Option<&'static str> {
    match encoded_register {
        0 if wide => Some(REG_NAME_AX),
        0 if !wide => Some(REG_NAME_AL),

        1 if wide => Some(REG_NAME_CX),
        1 if !wide => Some(REG_NAME_CL),

        2 if wide => Some(REG_NAME_DX),
        2 if !wide => Some(REG_NAME_DL),

        3 if wide => Some(REG_NAME_BX),
        3 if !wide => Some(REG_NAME_BL),

        4 if wide => Some(REG_NAME_SP),
        4 if !wide => Some(REG_NAME_AH),
        
        5 if wide => Some(REG_NAME_BP),
        5 if !wide => Some(REG_NAME_CH),

        6 if wide => Some(REG_NAME_SI),
        6 if !wide => Some(REG_NAME_DH),

        7 if wide => Some(REG_NAME_DI),
        7 if !wide => Some(REG_NAME_BH),

        _ => None
    }
}

fn get_op_name(encoded_op: u8) -> Option<&'static str> {
    match encoded_op {
        0b100010 => Some(OP_NAME_MOV),

        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Please provide an assembled 8086 instruction stream");
        process::exit(1);
    }

    let file = fs::File::open(&args[1]).unwrap_or_else(|_| panic!("Failed to open file {}", args[1]));
    let mut instruction_stream = BufReader::new(file);

    println!("bits 16\n"); // header just for diffing TODO remove?
    while !instruction_stream.fill_buf().expect("Failed to read instruction stream").is_empty() {
        let mut specifier = [0u8; 1];
        instruction_stream.read_exact(&mut specifier).expect("Failed to read op type");
        let specifier = unsafe { std::mem::transmute::<[u8; 1], u8>(specifier) };
        let op_type = specifier >> 2;
        let dest = (specifier & 0b10) >> 1 == 1;
        let wide = specifier & 0b01 == 1;

        let mut operands = [0u8; 1];
        instruction_stream.read_exact(&mut operands).expect("Failed to read mov operands");
        let operands = unsafe { std::mem::transmute::<[u8; 1], u8>(operands) };
        let mode = operands >> 6;
        let reg = (operands & 0b111000) >> 3;
        let reg_mem = operands & 0b111;

        let op_name = get_op_name(op_type).unwrap_or_else(|| panic!("{} is not a valid operation encoding", op_type));
        let dest_name = get_register_name(if dest { reg } else { reg_mem }, wide).unwrap_or_else(|| panic!("{} or {} is not a valid register encoding", reg, reg_mem));
        let source_name = get_register_name(if dest { reg_mem } else { reg }, wide).unwrap_or_else(|| panic!("{} or {} is not a valid register encoding", reg, reg_mem));
        println!("{} {}, {}", op_name, dest_name, source_name);
    }
}
