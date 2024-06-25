use std::{
    env,
    process,
    fmt,
    fs::File,
    io::{ prelude::*, BufReader }
};

const REG_NAME_AX: &str = "ax"; const REG_NAME_AL: &str = "al"; const REG_NAME_AH: &str = "ah";
const REG_NAME_CX: &str = "cx"; const REG_NAME_CL: &str = "cl"; const REG_NAME_CH: &str = "ch";
const REG_NAME_DX: &str = "dx"; const REG_NAME_DL: &str = "dl"; const REG_NAME_DH: &str = "dh";
const REG_NAME_BX: &str = "bx"; const REG_NAME_BL: &str = "bl"; const REG_NAME_BH: &str = "bh";
const REG_NAME_SP: &str = "sp";
const REG_NAME_BP: &str = "bp";
const REG_NAME_SI: &str = "si";
const REG_NAME_DI: &str = "di";

const OP_NAME_MOV: &str = "mov";

fn get_register_name(reg: u8, wide: bool) -> Option<&'static str> {
    match reg {
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

fn get_memory_expression(mode: u8, reg_or_mem: u8, disp_lo: u8, disp_hi: u8) -> Option<String> {
    let mut is_disp_negative = false;
    let disp = match (disp_lo, disp_hi) {
        (0, 0) => 0,
        (lo, 0) => {
            is_disp_negative = lo.rotate_left(1) & 1 == 1;
            (if is_disp_negative { lo.wrapping_neg() } else { lo }) as u16
        },
        (lo, hi) => {
            is_disp_negative = hi.rotate_left(1) & 1 == 1;
            let val = (hi as u16) << 8 | lo as u16;
            if is_disp_negative { val.wrapping_neg() } else { val }
        }
    };
    let disp_op = if is_disp_negative { "-" } else { "+" };

    match reg_or_mem {
        0b000 if disp != 0 => Some(format!(
            "[{} + {} {} {}]",
            REG_NAME_BX,
            REG_NAME_SI,
            disp_op,
            disp
        )),
        0b000 => Some(format!("[{} + {}]", REG_NAME_BX, REG_NAME_SI)),

        0b001 if disp != 0 => Some(format!(
            "[{} + {} {} {}]",
            REG_NAME_BX,
            REG_NAME_DI,
            disp_op,
            disp
        )),
        0b001 => Some(format!("[{} + {}]", REG_NAME_BX, REG_NAME_DI)),

        0b010 if disp != 0 => Some(format!(
            "[{} + {} {} {}]",
            REG_NAME_BP,
            REG_NAME_SI,
            disp_op,
            disp
        )),
        0b010 => Some(format!("[{} + {}]", REG_NAME_BP, REG_NAME_SI)),

        0b011 if disp != 0 => Some(format!(
            "[{} + {} {} {}]",
            REG_NAME_BP,
            REG_NAME_DI,
            disp_op,
            disp
        )),
        0b011 => Some(format!("[{} + {}]", REG_NAME_BP, REG_NAME_DI)),

        0b100 if disp != 0 => Some(format!(
            "[{} {} {}]",
            REG_NAME_SI,
            disp_op,
            disp
        )),
        0b100 => Some(format!("[{}]", REG_NAME_SI)),

        0b101 if disp != 0 => Some(format!(
            "[{} {} {}]",
            REG_NAME_DI,
            disp_op,
            disp
        )),
        0b101 => Some(format!("[{}]", REG_NAME_DI)),

        0b110 if mode == 0b00 => Some(format!("[{}]", disp)), // direct address
        0b110 if disp == 0 => Some(format!("[{}]", REG_NAME_BP)),
        0b110 => Some(format!(
            "[{} {} {}]",
            REG_NAME_BP,
            disp_op,
            disp
        )),

        0b111 if disp != 0 => Some(format!(
            "[{} {} {}]",
            REG_NAME_BX,
            disp_op,
            disp
        )),
        0b111 => Some(format!("[{}]", REG_NAME_BX)),

        _ => None
    }
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
enum Instruction {
    Mov_RegMem_ToFrom_Reg {
        dest: bool,
        wide: bool,
        mode: u8,
        reg: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
    },

    Mov_Imm_To_RegMem {
        wide: bool,
        mode: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
        data: u16,
    },

    Mov_Imm_To_Reg {
        wide: bool,
        reg: u8,
        data: u16,
    },

    Mov_Mem_To_Acc {
        wide: bool,
        address: u16,
    },

    Mov_Acc_To_Mem {
        wide: bool,
        address: u16,
    },

    Add_RegMem_With_Reg_to_Either {
        dest: bool,
        wide: bool,
        mode: u8,
        reg: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
    },

    Add_Imm_to_RegMem {
        sign_extend: bool,
        wide: bool,
        mode: u8,
        disp_lo: u8,
        disp_hi: u8,
        data: u16,
    },

    Add_Imm_To_Acc {
        wide: bool,
        data: u16,
    },

    Sub_RegMem_And_Reg_To_Either {
        dest: bool,
        wide: bool,
        mode: u8,
        reg: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
    },

    Sub_Imm_From_RegMem {
        sign_extend: bool,
        wide: bool,
        mode: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
        data: u16,
    },

    Sub_Imm_From_Acc {
        wide: bool,
        data: u16,
    },

    Cmp_RegMem_And_Reg {
        dest: bool,
        wide: bool,
        mode: u8,
        reg: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
    },

    Cmp_Imm_With_RegMem {
        sign_extend: bool,
        wide: bool,
        mode: u8,
        reg_or_mem: u8,
        disp_lo: u8,
        disp_hi: u8,
        data: u16,
    },

    Cmp_Imm_With_Acc {
        wide: bool,
        data: u8
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Instruction::Mov_RegMem_ToFrom_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi } => {
                let reg_name = get_register_name(reg, wide)
                    .unwrap_or("invalid register")
                    .to_string();
                let reg_name_or_mem_expr = if mode == 0b11 {
                    get_register_name(reg_or_mem, wide)
                        .unwrap_or("invalid register")
                        .to_string()
                } else {
                    get_memory_expression(mode, reg_or_mem, disp_lo, disp_hi)
                        .unwrap_or(String::from("[unrecognized address calculation expression]"))
                };

                if dest {
                    write!(formatter, "{} {}, {}", OP_NAME_MOV, reg_name, reg_name_or_mem_expr)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_MOV, reg_name_or_mem_expr, reg_name)
                }
            },

            Instruction::Mov_Imm_To_RegMem { wide, mode, reg_or_mem, disp_lo, disp_hi, data } => {
                let reg_name_or_mem_expr = if mode == 0b11 {
                    get_register_name(reg_or_mem, wide)
                        .unwrap_or("invalid register")
                        .to_string()
                } else {
                    get_memory_expression(mode, reg_or_mem, disp_lo, disp_hi)
                        .unwrap_or(String::from("[unrecognized address calculation expression]"))
                };

                if wide {
                    write!(formatter, "{} {}, word {}", OP_NAME_MOV, reg_name_or_mem_expr, data)
                } else {
                    write!(formatter, "{} {}, byte {}", OP_NAME_MOV, reg_name_or_mem_expr, data)
                }
            },

            Instruction::Mov_Imm_To_Reg { reg, data, wide } => {
                let reg_name = get_register_name(reg, wide)
                    .unwrap_or("invalid register")
                    .to_string();

                write!(formatter, "{} {}, {}", OP_NAME_MOV, reg_name, data)
            },

            Instruction::Mov_Mem_To_Acc { wide, address } => {
                let reg_name = if wide { REG_NAME_AX } else { REG_NAME_AL }.to_string();
                
                write!(formatter, "{} {}, [{}]", OP_NAME_MOV, reg_name, address)
            },

            Instruction::Mov_Acc_To_Mem { wide, address } => {
                let reg_name = if wide { REG_NAME_AX } else { REG_NAME_AL }.to_string();


                write!(formatter, "{} [{}], {}", OP_NAME_MOV, address, reg_name)
            }
        }
    }
}

fn read_byte(instruction_stream: &mut BufReader<File>) -> u8 {
    let mut byte = [0u8; 1];
    instruction_stream.read_exact(&mut byte).expect("Failed to read byte from instruction stream");
    unsafe { std::mem::transmute::<[u8; 1], u8>(byte) }
}

fn read_word(instruction_stream: &mut BufReader<File>) -> u16 {
    let mut word = [0u8; 2];
    instruction_stream.read_exact(&mut word).expect("Failed to read word from instruction stream");
    unsafe { std::mem::transmute::<[u8; 2], u16>(word) }
}

#[inline(always)]
fn read_displacement_bytes(instruction_stream: &mut BufReader<File>, mode: u8, reg_or_mem: u8) -> (u8, u8) {
    // returns (disp_lo, disp_hi)
    match mode {
        0b00 if reg_or_mem == 0b110 => (read_byte(instruction_stream), read_byte(instruction_stream)),
        0b10 => (read_byte(instruction_stream), read_byte(instruction_stream)),
        0b01 => (read_byte(instruction_stream), 0),
        _ => (0, 0),
    }
}

fn decode_instruction(instruction_stream: &mut BufReader<File>) -> Option<Instruction> {
    let byte = read_byte(instruction_stream);

    let opcode = byte >> 4;
    if opcode == 0b1011 {
        let wide = (byte >> 3) & 0b1 == 1;
        let reg = byte & 0b111;
        let data = if wide {
            read_word(instruction_stream)
        } else {
            read_byte(instruction_stream) as u16
        };

        return Some(Instruction::Mov_Imm_To_Reg { wide, reg, data })
    }

    let opcode = byte >> 2;
    if opcode == 0b100010 {
        let operands = read_byte(instruction_stream);

        let dest = (byte & 0b10) >> 1 == 1;
        let wide = byte & 0b01 == 1;
        let mode = operands >> 6;
        let reg = (operands & 0b111000) >> 3;
        let reg_or_mem = operands & 0b111;
        let (disp_lo, disp_hi) = read_displacement_bytes(instruction_stream, mode, reg_or_mem);

        return Some(Instruction::Mov_RegMem_ToFrom_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi });
    }

    let opcode = byte >> 1;
    match opcode {
        0b1100011 => {
            let operands = read_byte(instruction_stream);

            let wide = byte & 0b1 == 1;
            let mode = operands >> 6;
            let reg_or_mem = operands & 0b111;
            let (disp_lo, disp_hi) = read_displacement_bytes(instruction_stream, mode, reg_or_mem);
            let data = if wide {
                read_word(instruction_stream)
            } else {
                read_byte(instruction_stream) as u16
            };

            return Some(Instruction::Mov_Imm_To_RegMem { wide, mode, reg_or_mem, data, disp_lo, disp_hi });
        },

        0b1010000 => {
            let wide = byte & 0b1 == 1;
            let address = read_word(instruction_stream);

            return Some(Instruction::Mov_Mem_To_Acc { wide, address });
        },

        0b1010001 => {
            let wide = byte & 0b1 == 1;
            let address = read_word(instruction_stream);
            
            return Some(Instruction::Mov_Acc_To_Mem { wide, address });
        },
        _ => {}
    };

    None
}

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
