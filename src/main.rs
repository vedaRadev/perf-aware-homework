use std::{
    env,
    process,
    io::{ prelude::*, BufReader },
    fs::File,
};

mod decoder;
use decoder::*;

static mut REGISTERS: [u16; 8] = [0u16; 8];

fn set_high_byte(value: &mut u16, to: u8) {
    let ptr: *mut u16 = value;
    unsafe { *((ptr as *mut u8).offset(1)) = to };
}

fn set_low_byte(value: &mut u16, to: u8) {
    let ptr: *mut u16 = value;
    unsafe { *(ptr as *mut u8) = to };
}

fn get_register_value(encoding: u8, access: &RegisterAccess) -> u16 {
    unsafe {
        match access {
            RegisterAccess::Low => REGISTERS[encoding as usize].to_ne_bytes()[1] as u16,
            RegisterAccess::High => REGISTERS[encoding as usize].to_ne_bytes()[0] as u16,
            RegisterAccess::Full => REGISTERS[encoding as usize],
        }
    }
}

fn set_register_value(encoding: u8, access: &RegisterAccess, value: u16) {
    let register = unsafe { &mut REGISTERS[encoding as usize] };

    match access {
        RegisterAccess::Full => *register = value,
        RegisterAccess::High => set_high_byte(register, value as u8),
        RegisterAccess::Low => set_low_byte(register, value as u8),
    };
}

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
            match &instruction.operands {
                [ Some(destination), Some(source) ] => {
                    let source_value = match source {
                        Operand::Register(encoding, access) => get_register_value(*encoding, access),
                        Operand::ImmediateData(data) => *data,

                        Operand::Memory(_) => todo!(),
                        Operand::LabelOffset(_) => todo!(),
                    };

                    let destination_value_before;
                    let destination_value_after;

                    match instruction.operation {
                        Operation::Mov_RegMem_ToFrom_Reg
                        | Operation::Mov_Imm_To_Reg
                        | Operation::Mov_Imm_To_RegMem
                        | Operation::Mov_Mem_To_Acc
                        | Operation::Mov_Acc_To_Mem => {
                            match destination {
                                Operand::Register(encoding, access) => {
                                    destination_value_before = get_register_value(*encoding, access);
                                    set_register_value(*encoding, access, source_value);
                                    destination_value_after = get_register_value(*encoding, access);
                                },

                                Operand::Memory(_) => todo!(),

                                _ => panic!("cannot move into immediate or label offset"),
                            };

                            println!("{} ; {}: {} -> {}", instruction, destination, destination_value_before, destination_value_after);
                        },

                        _ => panic!("Invalid 2-operand instruction encountered")
                    };
                },

                [ Some(_), None ] => todo!("1-operand instructions not implemented"),

                [ None, None ] => todo!("0-operand instructions not implemented"),
                _ => panic!("invalid operand configuration [ None, Some(...) ]"),
            };
        }
    }

    println!("\nFinal register states:");
    unsafe {
        for (register_index, value) in REGISTERS.iter().enumerate() {
            println!("\t{}: {:#x}", get_register_name(register_index as u8, true).expect("Invalid register"), value);
        }
    }
}
