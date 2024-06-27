use std::{
    fmt,
    fs::File,
    io::{ prelude::*, BufReader },
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
const OP_NAME_ADD: &str = "add";
const OP_NAME_SUB: &str = "sub";
const OP_NAME_CMP: &str = "cmp";
const OP_NAME_JE: &str = "je";
const OP_NAME_JL: &str = "jl";
const OP_NAME_JLE: &str = "jle";
const OP_NAME_JB : &str = "jb";
const OP_NAME_JBE: &str = "jbe";
const OP_NAME_JP: &str = "jp";
const OP_NAME_JO: &str = "jo";
const OP_NAME_JS: &str = "js";
const OP_NAME_JNE: &str = "jne";
const OP_NAME_JNL: &str = "jnl";
const OP_NAME_JG: &str = "jg";
const OP_NAME_JNB: &str = "jnb";
const OP_NAME_JA: &str = "ja";
const OP_NAME_JNP: &str = "jnp";
const OP_NAME_JNO: &str = "jno";
const OP_NAME_JNS: &str = "jns";
const OP_NAME_LOOP: &str = "loop";
const OP_NAME_LOOPZ: &str = "loopz";
const OP_NAME_LOOPNZ: &str = "loopnz";
const OP_NAME_JCXZ: &str = "jcxz";

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
pub enum Instruction {
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
        reg_or_mem: u8,
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
        data: u16
    },

    Jmp_On_Equal { inc: u8 }, // je
    Jmp_On_Less { inc: u8 }, // jl
    Jmp_On_Less_Or_Equal { inc: u8 }, // jle
    Jmp_On_Below { inc: u8 }, // jb
    Jmp_On_Below_Or_Equal { inc: u8 }, // jbe
    Jmp_On_Greater { inc: u8 }, // jg
    Jmp_On_Above { inc: u8 }, // ja
    Jmp_On_Parity { inc: u8 }, // jp
    Jmp_On_Overflow { inc: u8 }, // jo
    Jmp_On_Sign { inc: u8 }, // js
    Jmp_On_Not_Equal { inc: u8 }, // jne
    Jmp_On_Not_Less { inc: u8 }, // jnl
    Jmp_On_Not_Below { inc: u8 }, // jnb
    Jmp_On_Not_Parity { inc: u8 }, // jnp
    Jmp_On_Not_Overflow { inc: u8 }, // jno
    Jmp_On_Not_Sign { inc: u8 }, // jns
    Jmp_On_CX_Zero { inc: u8 }, // jcxz
    
    Loop { inc: u8 }, // loop
    Loop_While_Zero { inc: u8 }, // joopz
    Loop_While_Not_Zero { inc: u8 }, // loopnz
}

fn get_formatted_jmp_loop_instruction(formatter: &mut fmt::Formatter, op_name: &str, inc: u8) -> fmt::Result {
    if inc.rotate_left(1) & 1 == 1 {
        write!(formatter, "{} -{}", op_name, inc.wrapping_neg())
    } else {
        write!(formatter, "{} {}", op_name, inc)
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {

            //***************
            // MOV
            //***************

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
            },

            //***************
            // ADD
            //***************

            Instruction::Add_RegMem_With_Reg_to_Either { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi } => {
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
                    write!(formatter, "{} {}, {}", OP_NAME_ADD, reg_name, reg_name_or_mem_expr)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_ADD, reg_name_or_mem_expr, reg_name)
                }
            },

            Instruction::Add_Imm_to_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data } => {
                let is_reg_mode = mode == 0b11;
                let reg_name_or_mem_expr = if is_reg_mode {
                    get_register_name(reg_or_mem, wide)
                        .unwrap_or("invalid register")
                        .to_string()
                } else {
                    get_memory_expression(mode, reg_or_mem, disp_lo, disp_hi)
                        .unwrap_or(String::from("[unrecognized address calculation expression]"))
                };

                if is_reg_mode {
                    write!(formatter, "{} {}, {}", OP_NAME_ADD, reg_name_or_mem_expr, data)
                } else if sign_extend && wide {
                    write!(formatter, "{} {}, word {}", OP_NAME_ADD, reg_name_or_mem_expr, data)
                } else {
                    write!(formatter, "{} {}, byte {}", OP_NAME_ADD, reg_name_or_mem_expr, data)
                }
            },

            Instruction::Add_Imm_To_Acc { wide, data } => {
                if wide {
                    write!(formatter, "{} {}, {}", OP_NAME_ADD, REG_NAME_AX, data)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_ADD, REG_NAME_AL, data)
                }
            },

            //***************
            // SUB
            //***************

            Instruction::Sub_RegMem_And_Reg_To_Either { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi } => {
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
                    write!(formatter, "{} {}, {}", OP_NAME_SUB, reg_name, reg_name_or_mem_expr)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_SUB, reg_name_or_mem_expr, reg_name)
                }
            },

            Instruction::Sub_Imm_From_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data } => {
                let is_reg_mode = mode == 0b11;
                let reg_name_or_mem_expr = if is_reg_mode {
                    get_register_name(reg_or_mem, wide)
                        .unwrap_or("invalid register")
                        .to_string()
                } else {
                    get_memory_expression(mode, reg_or_mem, disp_lo, disp_hi)
                        .unwrap_or(String::from("[unrecognized address calculation expression]"))
                };

                if is_reg_mode {
                    write!(formatter, "{} {}, {}", OP_NAME_SUB, reg_name_or_mem_expr, data)
                } else if sign_extend && wide {
                    write!(formatter, "{} {}, word {}", OP_NAME_SUB, reg_name_or_mem_expr, data)
                } else {
                    write!(formatter, "{} {}, byte {}", OP_NAME_SUB, reg_name_or_mem_expr, data)
                }
            },

            Instruction::Sub_Imm_From_Acc { wide, data } => {
                if wide {
                    write!(formatter, "{} {}, {}", OP_NAME_SUB, REG_NAME_AX, data)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_SUB, REG_NAME_AL, data)
                }
            },

            //***************
            // CMP
            //***************

            Instruction::Cmp_RegMem_And_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi } => {
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
                    write!(formatter, "{} {}, {}", OP_NAME_CMP, reg_name, reg_name_or_mem_expr)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_CMP, reg_name_or_mem_expr, reg_name)
                }
            },

            Instruction::Cmp_Imm_With_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data } => {
                let is_reg_mode = mode == 0b11;
                let reg_name_or_mem_expr = if is_reg_mode {
                    get_register_name(reg_or_mem, wide)
                        .unwrap_or("invalid register")
                        .to_string()
                } else {
                    get_memory_expression(mode, reg_or_mem, disp_lo, disp_hi)
                        .unwrap_or(String::from("[unrecognized address calculation expression]"))
                };

                if is_reg_mode {
                    write!(formatter, "{} {}, {}", OP_NAME_CMP, reg_name_or_mem_expr, data)
                } else if sign_extend && wide {
                    write!(formatter, "{} {}, word {}", OP_NAME_CMP, reg_name_or_mem_expr, data)
                } else {
                    write!(formatter, "{} {}, byte {}", OP_NAME_CMP, reg_name_or_mem_expr, data)
                }
            },

            Instruction::Cmp_Imm_With_Acc { wide, data } => {
                if wide {
                    write!(formatter, "{} {}, {}", OP_NAME_CMP, REG_NAME_AX, data)
                } else {
                    write!(formatter, "{} {}, {}", OP_NAME_CMP, REG_NAME_AL, data)
                }
            }

            //***************
            // JMP / LOOP
            //***************

            Instruction::Jmp_On_Equal { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JE, inc),
            Instruction::Jmp_On_Less { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JL, inc),
            Instruction::Jmp_On_Less_Or_Equal { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JLE, inc),
            Instruction::Jmp_On_Below { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JB, inc),
            Instruction::Jmp_On_Below_Or_Equal { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JBE, inc),
            Instruction::Jmp_On_Greater { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JG, inc),
            Instruction::Jmp_On_Above { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JA, inc),
            Instruction::Jmp_On_Parity { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JP, inc),
            Instruction::Jmp_On_Overflow { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JO, inc),
            Instruction::Jmp_On_Sign { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JS, inc),
            Instruction::Jmp_On_Not_Equal { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNE, inc),
            Instruction::Jmp_On_Not_Less { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNL, inc),
            Instruction::Jmp_On_Not_Below { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNB, inc),
            Instruction::Jmp_On_Not_Parity { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNP, inc),
            Instruction::Jmp_On_Not_Overflow { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNO, inc),
            Instruction::Jmp_On_Not_Sign { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JNS, inc),
            Instruction::Jmp_On_CX_Zero { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_JCXZ, inc),
            Instruction::Loop { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_LOOP, inc),
            Instruction::Loop_While_Zero { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_LOOPZ, inc),
            Instruction::Loop_While_Not_Zero { inc } => get_formatted_jmp_loop_instruction(formatter, OP_NAME_LOOPNZ, inc),
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

#[inline(always)]
fn read_data(instruction_stream: &mut BufReader<File>, wide: bool) -> u16 {
    if wide {
        read_word(instruction_stream)
    } else {
        read_byte(instruction_stream) as u16
    }
}

type Opcode6BitData = (bool, bool, u8, u8, u8, u8, u8);
// TODO rename this as it's used for more than just 6-bit opcodes (at least once for 7-bit)
fn get_6bit_opcode_instruction_data(instruction_stream: &mut BufReader<File>, opcode_byte: u8) -> Opcode6BitData {
    let operands = read_byte(instruction_stream);
    let flag_1 = (opcode_byte & 0b10) >> 1 == 1;
    let flag_2 = opcode_byte & 0b01 == 1;
    let mode = operands >> 6;
    let reg_or_subopcode = (operands & 0b111000) >> 3;
    let reg_or_mem = operands & 0b111;
    let (disp_lo, disp_hi) = read_displacement_bytes(instruction_stream, mode, reg_or_mem);

    (flag_1, flag_2, mode, reg_or_subopcode, reg_or_mem, disp_lo, disp_hi)
}

pub fn decode_instruction(instruction_stream: &mut BufReader<File>) -> Option<Instruction> {
    let byte = read_byte(instruction_stream);

    let opcode = byte >> 4;
    if opcode == 0b1011 {
        let wide = (byte >> 3) & 0b1 == 1;
        let reg = byte & 0b111;
        let data = read_data(instruction_stream, wide);

        return Some(Instruction::Mov_Imm_To_Reg { wide, reg, data })
    }

    let opcode = byte >> 2;
    match opcode {
        0b100010 => {
            let (dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            return Some(Instruction::Mov_RegMem_ToFrom_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi });
        },

        0b000000 => {
            let (dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            return Some(Instruction::Add_RegMem_With_Reg_to_Either { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi });
        },

        0b001010 => {
            let (dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            return Some(Instruction::Sub_RegMem_And_Reg_To_Either { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi });
        },

        0b01110 => {
            let (dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            return Some(Instruction::Cmp_RegMem_And_Reg { dest, wide, mode, reg, reg_or_mem, disp_lo, disp_hi });
        },

        0b100000 => {
            let (sign_extend, wide, mode, sub_opcode, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            // let data = read_data(instruction_stream, wide); 
            let data = if !sign_extend && wide { read_word(instruction_stream) } else { read_byte(instruction_stream) as u16 };

            return match sub_opcode {
                0b000 => Some(Instruction::Add_Imm_to_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data }),
                0b101 => Some(Instruction::Sub_Imm_From_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data }),
                0b111 => Some(Instruction::Cmp_Imm_With_RegMem { sign_extend, wide, mode, reg_or_mem, disp_lo, disp_hi, data }),
                _ => None,
            };
        },

        _ => {},
    }

    let opcode = byte >> 1;
    match opcode {
        0b1100011 => {
            let (_, wide, mode, _, reg_or_mem, disp_lo, disp_hi) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let data = read_data(instruction_stream, wide);

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

        0b0000010 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            return Some(Instruction::Add_Imm_To_Acc { wide, data });
        },

        0b10110 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            return Some(Instruction::Sub_Imm_From_Acc { wide, data });
        },

        0b011110 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            return Some(Instruction::Cmp_Imm_With_Acc { wide, data });
        },

        _ => {}
    };

    let opcode = byte;
    match opcode {
        0b01110100 => return Some(Instruction::Jmp_On_Equal { inc: read_byte(instruction_stream) }),
        0b01111100 => return Some(Instruction::Jmp_On_Less { inc: read_byte(instruction_stream) }),
        0b01111110 => return Some(Instruction::Jmp_On_Less_Or_Equal { inc: read_byte(instruction_stream) }),
        0b01110010 => return Some(Instruction::Jmp_On_Below { inc: read_byte(instruction_stream) }),
        0b01110110 => return Some(Instruction::Jmp_On_Below_Or_Equal { inc: read_byte(instruction_stream) }),
        0b01111111 => return Some(Instruction::Jmp_On_Greater { inc: read_byte(instruction_stream) }),
        0b01110111 => return Some(Instruction::Jmp_On_Above { inc: read_byte(instruction_stream) }),
        0b01111010 => return Some(Instruction::Jmp_On_Parity { inc: read_byte(instruction_stream) }),
        0b01110000 => return Some(Instruction::Jmp_On_Overflow { inc: read_byte(instruction_stream) }),
        0b01111000 => return Some(Instruction::Jmp_On_Sign { inc: read_byte(instruction_stream) }),
        0b01110101 => return Some(Instruction::Jmp_On_Not_Equal { inc: read_byte(instruction_stream) }),
        0b01111101 => return Some(Instruction::Jmp_On_Not_Less { inc: read_byte(instruction_stream) }),
        0b01110011 => return Some(Instruction::Jmp_On_Not_Below { inc: read_byte(instruction_stream) }),
        0b01111011 => return Some(Instruction::Jmp_On_Not_Parity { inc: read_byte(instruction_stream) }),
        0b01110001 => return Some(Instruction::Jmp_On_Not_Overflow { inc: read_byte(instruction_stream) }),
        0b01111001 => return Some(Instruction::Jmp_On_Not_Sign { inc: read_byte(instruction_stream) }),
        0b11100011 => return Some(Instruction::Jmp_On_CX_Zero { inc: read_byte(instruction_stream) }),
        0b11100010 => return Some(Instruction::Loop { inc: read_byte(instruction_stream) }),
        0b11100001 => return Some(Instruction::Loop_While_Zero { inc: read_byte(instruction_stream) }),
        0b11100000 => return Some(Instruction::Loop_While_Not_Zero { inc: read_byte(instruction_stream) }),
        _ => {}
    };

    None
}
