use std::{
    env,
    process,
    io::{ prelude::*, BufReader },
    fs::File,
};

mod decoder;
use decoder::{ decode_instruction, Instruction, get_register_name };

enum RegisterAccess { Low, High, Full }
fn get_register_index_and_access(register_encoding: u8, wide: bool) -> (usize, RegisterAccess) {
    // from intel 8086 manual: the register table
    // REG  W=0 W=1
    // 000  AL  AX
    // 001  CL  CX
    // 010  DL  DX
    // 011  BL  BX
    // 100  AH  SP
    // 101  CH  BP
    // 110  DH  SI
    // 111  BH  DI
    if wide {
        (register_encoding as usize, RegisterAccess::Full)
    } else {
        let index = (register_encoding & 0b11) as usize;
        let access = if register_encoding & 0b100 == 0 { RegisterAccess::Low } else { RegisterAccess::High };
        (index, access)
    }
}

fn set_high_byte(value: &mut u16, to: u8) {
    let ptr: *mut u16 = value;
    unsafe { *((ptr as *mut u8).offset(1)) = to };
}

fn set_low_byte(value: &mut u16, to: u8) {
    let ptr: *mut u16 = value;
    unsafe { *(ptr as *mut u8) = to };
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

    let mut registers = [0u16; 8];

    println!("bits 16\n"); // header needed to specify 16-bit wide registers
    while !instruction_stream.fill_buf().expect("Failed to read instruction stream").is_empty() {
        if let Some(instruction) = decode_instruction(&mut instruction_stream) {
            match instruction {
                #[allow(unused_variables)]
                Instruction::Mov_RegMem_ToFrom_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi } => {
                    if mode != 0b11 { todo!("memory mode mov not implemented yet"); }

                    let src_reg;
                    let dest_reg;
                    if dest { dest_reg = reg; src_reg = reg_or_mem; } else { dest_reg = reg_or_mem; src_reg = reg; }
                    let (dest_index, dest_access) = get_register_index_and_access(dest_reg, wide);
                    let (src_index, src_access) = get_register_index_and_access(src_reg, wide);

                    let src_val: u16 = match src_access {
                        RegisterAccess::Low => registers[src_index].to_ne_bytes()[1] as u16,
                        RegisterAccess::High => registers[src_index].to_ne_bytes()[0] as u16,
                        RegisterAccess::Full => registers[src_index],
                    };

                    let reg_val_before = registers[dest_index];
                    match dest_access {
                        RegisterAccess::Low => set_low_byte(&mut registers[dest_index], src_val as u8),
                        RegisterAccess::High => set_high_byte(&mut registers[dest_index], src_val as u8),
                        RegisterAccess::Full => registers[dest_index] = src_val,
                    };
                    let reg_val_after = registers[dest_index];

                    println!(
                        "{} ; {}: {:#x} -> {:#x}",
                        instruction,
                        get_register_name(dest_reg, wide).expect("invalid register"),
                        reg_val_before,
                        reg_val_after,
                    );
                },
                
                #[allow(unused_variables)]
                Instruction::Mov_Imm_To_RegMem { wide, mode, reg_or_mem, disp_lo, disp_hi, data } => todo!(),

                instruction @ Instruction::Mov_Imm_To_Reg { wide, reg, data } => {
                    let (index, access) = get_register_index_and_access(reg, wide);
                    let reg_val_before = registers[index];
                    match access {
                        RegisterAccess::Low => set_low_byte(&mut registers[index], data as u8),
                        RegisterAccess::High => set_high_byte(&mut registers[index], data as u8),
                        RegisterAccess::Full => registers[index] = data,
                    };
                    let reg_val_after = registers[index];

                    println!(
                        "{} ; {}: {:#x} -> {:#x}",
                        instruction,
                        get_register_name(reg, wide).expect("Invalid register"),
                        reg_val_before,
                        reg_val_after
                    );
                },

                #[allow(unused_variables)]
                Instruction::Mov_Mem_To_Acc { wide, address } => todo!(),

                #[allow(unused_variables)]
                Instruction::Mov_Acc_To_Mem { wide, address } => todo!(),

                _ => todo!(),
            };
        }
    }

    println!("\nFinal register states:");
    for (register_index, value) in registers.iter().enumerate() {
        println!("\t{}: {:#x}", get_register_name(register_index as u8, true).expect("Invalid register"), value);
    }
}
