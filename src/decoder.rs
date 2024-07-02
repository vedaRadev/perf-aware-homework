use std::{
    fmt,
    io::{ prelude::*, Cursor },
};

const REGISTER_NAMES: [[&str; 2]; 8] = [
    ["al", "ax"],
    ["cl", "cx"],
    ["dl", "dx"],
    ["bl", "bx"],
    ["ah", "sp"],
    ["ch", "bp"],
    ["dh", "si"],
    ["bh", "di"],
];

pub fn get_register_name(reg: u8, wide: bool) -> Option<&'static str> {
    if reg > 7 { None } else { Some(REGISTER_NAMES[reg as usize][wide as usize]) }
}

pub enum RegisterAccess { Low, High, Full, }

impl RegisterAccess {
    fn new(encoded_register: u8, wide: bool) -> Self {
        if wide {
            RegisterAccess::Full
        } else if (encoded_register & 0b100) >> 2 == 1 {
            RegisterAccess::High
        } else {
            RegisterAccess::Low
        }
    }
}

#[allow(non_camel_case_types)]
pub enum EffectiveAddressBase {
    BX_SI,
    BX_DI,
    BP_SI,
    BP_DI,
    SI,
    DI,
    BP,
    BX,
}

impl fmt::Display for EffectiveAddressBase {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EffectiveAddressBase::BX_SI => write!(formatter, "bx + si"),
            EffectiveAddressBase::BX_DI => write!(formatter, "bx + di"),
            EffectiveAddressBase::BP_SI => write!(formatter, "bp + si"),
            EffectiveAddressBase::BP_DI => write!(formatter, "bp + di"),
            EffectiveAddressBase::SI => write!(formatter, "si"),
            EffectiveAddressBase::DI => write!(formatter, "di"),
            EffectiveAddressBase::BP => write!(formatter, "bp"),
            EffectiveAddressBase::BX => write!(formatter, "bx"),
        }
    }
}

// TODO maybe instead of having an effective address base we just store the two registers we're
// using since we can pull the encodings right from them?
pub enum EffectiveAddress {
    Direct(u16),
    Calculated { base: EffectiveAddressBase, displacement: u16 },
}

impl EffectiveAddress {
    fn new(mode: u8, encoding: u8, displacement: u16) -> Self {
        if mode == 0 && encoding == 0b110 {
            Self::Direct(displacement)
        } else {
            let base = match encoding {
                0b000 => EffectiveAddressBase::BX_SI,
                0b001 => EffectiveAddressBase::BX_DI,
                0b010 => EffectiveAddressBase::BP_SI,
                0b011 => EffectiveAddressBase::BP_DI,
                0b100 => EffectiveAddressBase::SI,
                0b101 => EffectiveAddressBase::DI,
                0b110 => EffectiveAddressBase::BP,
                0b111 => EffectiveAddressBase::BX,
                _ => panic!("Invalid effective address encoding: {:#b}", encoding)
            };

            Self::Calculated { base, displacement }
        }
    }
}

impl fmt::Display for EffectiveAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Direct(address) => write!(formatter, "[{}]", address),
            Self::Calculated { base, displacement } => {
                let displacement = *displacement;
                if displacement == 0 { return write!(formatter, "[{}]", base); }

                let [disp_lo, disp_hi] = displacement.to_ne_bytes();
                let is_wide = disp_hi != 0;
                let is_disp_negative = if is_wide { disp_hi.rotate_left(1) & 1 == 1 } else { disp_lo.rotate_left(1) & 1 == 1 };
                let disp_sign = if is_disp_negative { "-" } else { "+" };
                let disp_display_val = if is_disp_negative {
                    if is_wide { displacement.wrapping_neg() } else { disp_lo.wrapping_neg() as u16 }
                } else {
                    displacement
                };

                write!(formatter, "[{} {} {}]", base, disp_sign, disp_display_val)
            }
        }
    }
}

// AUDIT Do I really want to store the encoded register in here?
// Maybe I could create an enum named Reg with values:
// A, B, C, D, SP, BP, SI, DI
// Then the RegisterOperand could store the Register itself.
// Or I could do vice versa where the Register stores the RegisterAccess, then I could code in some
// invariants to say that SP, BP, SI, DI can only ever be full access.
pub enum Operand {
    Register(u8, RegisterAccess),
    Memory(EffectiveAddress),
    ImmediateData(u16),
    LabelOffset(i8), // instruction pointer increment
}

impl fmt::Display for Operand {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Operand::Register(encoding, register_access) => write!(formatter, "{}", match register_access {
                RegisterAccess::Full => get_register_name(*encoding, true).unwrap(),
                RegisterAccess::High => get_register_name(*encoding, false).unwrap(),
                RegisterAccess::Low => get_register_name(*encoding, false).unwrap(),
            }),
            Operand::Memory(effective_address) => write!(formatter, "{}", effective_address),
            Operand::ImmediateData(data) => write!(formatter, "{}", data),
            Operand::LabelOffset(offset) => write!(formatter, "{}", offset),
        }
    }
}

// AUDIT Can I get rid of some of these flags? Are some only useful during decoding?
#[allow(dead_code)]
#[derive(PartialEq)]
pub enum OperationFlag {
    SignExtension, // S
    Wide, // W
    Destination, // D
    Overflow, // V
    Zero, // Z
}

#[allow(non_camel_case_types)]
pub enum Operation {
    Mov_RegMem_ToFrom_Reg,
    Mov_Imm_To_RegMem,
    Mov_Imm_To_Reg,
    Mov_Mem_To_Acc,
    Mov_Acc_To_Mem,

    Add_RegMem_With_Reg_to_Either,
    Add_Imm_to_RegMem,
    Add_Imm_To_Acc,

    Sub_RegMem_And_Reg_To_Either,
    Sub_Imm_From_RegMem,
    Sub_Imm_From_Acc,

    Cmp_RegMem_And_Reg,
    Cmp_Imm_With_RegMem,
    Cmp_Imm_With_Acc,

    Jmp_On_Equal, // je
    Jmp_On_Less, // jl
    Jmp_On_Less_Or_Equal, // jle
    Jmp_On_Below, // jb
    Jmp_On_Below_Or_Equal, // jbe
    Jmp_On_Greater, // jg
    Jmp_On_Above, // ja
    Jmp_On_Parity, // jp
    Jmp_On_Overflow, // jo
    Jmp_On_Sign, // js
    Jmp_On_Not_Equal, // jne
    Jmp_On_Not_Less, // jnl
    Jmp_On_Not_Below, // jnb
    Jmp_On_Not_Parity, // jnp
    Jmp_On_Not_Overflow, // jno
    Jmp_On_Not_Sign, // jns
    Jmp_On_CX_Zero, // jcxz
    
    Loop, // loop
    Loop_While_Zero, // loopz
    Loop_While_Not_Zero, // loopnz
}

pub struct Instruction {
    pub operation: Operation,
    pub operands: [Option<Operand>; 2], // e.g. opcode operand_1, operand_2 (max 2 operands)
    pub flags: Vec<OperationFlag>, // really would like this to just be u8
}

impl Instruction {
    // TODO maybe find a way to use actual bitflags
    #[inline(always)]
    fn has_flag(&self, flag: OperationFlag) -> bool { self.flags.contains(&flag) }
}

impl fmt::Display for Instruction {
    #[allow(unused_variables)]
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let op_name = match self.operation {
            Operation::Mov_RegMem_ToFrom_Reg
            | Operation::Mov_Imm_To_RegMem
            | Operation::Mov_Imm_To_Reg
            | Operation::Mov_Mem_To_Acc
            | Operation::Mov_Acc_To_Mem
                => "mov",

            Operation::Add_RegMem_With_Reg_to_Either
            | Operation::Add_Imm_to_RegMem
            | Operation::Add_Imm_To_Acc
                => "add",

            Operation::Sub_RegMem_And_Reg_To_Either
            | Operation::Sub_Imm_From_RegMem
            | Operation::Sub_Imm_From_Acc
                => "sub",

            Operation::Cmp_RegMem_And_Reg
            | Operation::Cmp_Imm_With_RegMem
            | Operation::Cmp_Imm_With_Acc
                => "cmp",
            
            Operation::Jmp_On_Equal => "je",
            Operation::Jmp_On_Less => "jl",
            Operation::Jmp_On_Less_Or_Equal => "jle",
            Operation::Jmp_On_Below =>  "jb",
            Operation::Jmp_On_Below_Or_Equal =>  "jbe",
            Operation::Jmp_On_Greater =>  "jg",
            Operation::Jmp_On_Above =>  "ja",
            Operation::Jmp_On_Parity =>  "jp",
            Operation::Jmp_On_Overflow =>  "jo",
            Operation::Jmp_On_Sign =>  "js",
            Operation::Jmp_On_Not_Equal =>  "jne",
            Operation::Jmp_On_Not_Less =>  "jnl",
            Operation::Jmp_On_Not_Below =>  "jnb",
            Operation::Jmp_On_Not_Parity =>  "jnp",
            Operation::Jmp_On_Not_Overflow =>  "jno",
            Operation::Jmp_On_Not_Sign =>  "jns",
            Operation::Jmp_On_CX_Zero =>  "jcxz",

            Operation::Loop =>  "loop",
            Operation::Loop_While_Zero =>  "loopz",
            Operation::Loop_While_Not_Zero =>  "loopnz",
        };

        match &self.operands {
            [ None, None ] => todo!("printing for 0-operand instructions not implemented"),
            [ Some(operand), None ] => write!(formatter, "{} {}", op_name, operand),
            [ Some(dst @ Operand::Memory(_)), Some(src @ Operand::ImmediateData(data)) ] => {
                let size_specifier = if self.has_flag(OperationFlag::Wide) { "word" } else { "byte" };
                write!(formatter, "{} {}, {} {}", op_name, dst, size_specifier, src)
            },
            [ Some(dst), Some(src) ] => write!(formatter, "{} {}, {}", op_name, dst, src),
            _ => panic!("invalid operand configuration [None, Some(...)]")
        }
    }
}

fn read_byte(instruction_stream: &mut Cursor<Vec<u8>>) -> u8 {
    let mut byte = [0u8; 1];
    instruction_stream.read_exact(&mut byte).expect("Failed to read byte from instruction stream");
    unsafe { std::mem::transmute::<[u8; 1], u8>(byte) }
}

fn read_word(instruction_stream: &mut Cursor<Vec<u8>>) -> u16 {
    let mut word = [0u8; 2];
    instruction_stream.read_exact(&mut word).expect("Failed to read word from instruction stream");
    unsafe { std::mem::transmute::<[u8; 2], u16>(word) }
}

fn read_displacement_bytes(instruction_stream: &mut Cursor<Vec<u8>>, mode: u8, reg_or_mem: u8) -> u16 {
    match mode {
        0b00 if reg_or_mem == 0b110 => read_word(instruction_stream),
        0b10 => read_word(instruction_stream),
        0b01 => read_byte(instruction_stream) as u16,
        _ => 0
    }
}

#[inline(always)]
fn read_data(instruction_stream: &mut Cursor<Vec<u8>>, wide: bool) -> u16 {
    if wide {
        read_word(instruction_stream)
    } else {
        read_byte(instruction_stream) as u16
    }
}

type Opcode6BitData = (bool, bool, u8, u8, u8, u16);
// TODO rename this as it's used for more than just 6-bit opcodes (at least once for 7-bit)
fn get_6bit_opcode_instruction_data(instruction_stream: &mut Cursor<Vec<u8>>, opcode_byte: u8) -> Opcode6BitData {
    let operands = read_byte(instruction_stream);
    let flag_1 = (opcode_byte & 0b10) >> 1 == 1;
    let flag_2 = opcode_byte & 0b01 == 1;
    let mode = operands >> 6;
    let reg_or_subopcode = (operands & 0b111000) >> 3;
    let reg_or_mem = operands & 0b111;
    let displacement = read_displacement_bytes(instruction_stream, mode, reg_or_mem);

    (flag_1, flag_2, mode, reg_or_subopcode, reg_or_mem, displacement)
}

// TODO change the way we're decoding so that we can stop duplicating so much code
pub fn decode_instruction(instruction_stream: &mut Cursor<Vec<u8>>) -> Option<Instruction> {
    let byte = read_byte(instruction_stream);

    let opcode = byte >> 4;
    if opcode == 0b1011 {
        let wide = (byte >> 3) & 0b1 == 1;
        let reg = byte & 0b111;
        let data = read_data(instruction_stream, wide);

        let mut flags: Vec<OperationFlag> = vec![];
        if wide { flags.push(OperationFlag::Wide); }

        let dest_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
        let src_operand = Operand::ImmediateData(data);

        return Some(Instruction {
            operation: Operation::Mov_Imm_To_Reg,
            operands: [ Some(dest_operand), Some(src_operand) ],
            flags,
        });
    }

    let opcode = byte >> 2;
    match opcode {
        0b100010 => {
            let (dest, wide, mode, reg, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }
            if dest { flags.push(OperationFlag::Destination); }

            let register_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let other_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };

            let operands = if dest {
                [ Some(register_operand), Some(other_operand) ]
            } else {
                [ Some(other_operand), Some(register_operand) ]
            };

            return Some(Instruction {
                operands,
                operation: Operation::Mov_RegMem_ToFrom_Reg,
                flags,
            });
        },

        0b000000 => {
            let (dest, wide, mode, reg, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }
            if dest { flags.push(OperationFlag::Destination); }

            let register_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let other_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };

            let operands = if dest {
                [ Some(register_operand), Some(other_operand) ]
            } else {
                [ Some(other_operand), Some(register_operand) ]
            };

            return Some(Instruction {
                operation: Operation::Add_RegMem_With_Reg_to_Either,
                operands,
                flags,
            });
        },

        0b001010 => {
            let (dest, wide, mode, reg, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }
            if dest { flags.push(OperationFlag::Destination); }

            let register_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let other_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };

            let operands = if dest {
                [ Some(register_operand), Some(other_operand) ]
            } else {
                [ Some(other_operand), Some(register_operand) ]
            };

            return Some(Instruction {
                operation: Operation::Sub_RegMem_And_Reg_To_Either,
                operands,
                flags,
            });
        },

        0b01110 => {
            let (dest, wide, mode, reg, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }
            if dest { flags.push(OperationFlag::Destination); }

            let register_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let other_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };

            let operands = if dest {
                [ Some(register_operand), Some(other_operand) ]
            } else {
                [ Some(other_operand), Some(register_operand) ]
            };

            return Some(Instruction {
                operation: Operation::Cmp_RegMem_And_Reg,
                operands,
                flags,
            });
        },

        0b100000 => {
            let (sign_extend, wide, mode, sub_opcode, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let data = if !sign_extend && wide { read_word(instruction_stream) } else { read_byte(instruction_stream) as u16 };

            let mut flags: Vec<OperationFlag> = vec![];
            if sign_extend { flags.push(OperationFlag::SignExtension); }
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };
            let src_operand = Operand::ImmediateData(data);

            let operands = [ Some(dest_operand), Some(src_operand) ];

            return match sub_opcode {
                0b000 => Some(Instruction { operation: Operation::Add_Imm_to_RegMem, operands, flags }),
                0b101 => Some(Instruction { operation: Operation::Sub_Imm_From_RegMem, operands, flags }),
                0b111 => Some(Instruction { operation: Operation::Cmp_Imm_With_RegMem, operands, flags }),
                _ => None,
            };
        },

        _ => {},
    }

    let opcode = byte >> 1;
    match opcode {
        0b1100011 => {
            // HACK for simplicity, calculating 6bit opcode stuff here even though we're a 7bit
            // opcode
            let (_, wide, mode, _, reg_or_mem, displacement) = get_6bit_opcode_instruction_data(instruction_stream, byte);
            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }
            let dest_operand = if mode == 0b11 {
                Operand::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
            } else {
                Operand::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
            };
            let src_operand = Operand::ImmediateData(read_data(instruction_stream, wide));

            return Some(Instruction {
                operation: Operation::Mov_Imm_To_RegMem,
                operands: [ Some(dest_operand), Some(src_operand) ],
                flags
            });
        },

        0b1010000 => {
            let wide = byte & 0b1 == 1;
            let address = read_word(instruction_stream);

            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = Operand::Register(0b000, RegisterAccess::new(0b000 /* register A */, wide));
            let src_operand = Operand::Memory(EffectiveAddress::Direct(address));

            return Some(Instruction {
                operation: Operation::Mov_Mem_To_Acc,
                operands: [ Some(dest_operand), Some(src_operand) ],
                flags,
            });
        },

        0b1010001 => {
            let wide = byte & 0b1 == 1;
            let address = read_word(instruction_stream);

            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = Operand::Memory(EffectiveAddress::Direct(address));
            let src_operand = Operand::Register(0b000, RegisterAccess::new(0b000 /* register A */, wide));

            return Some(Instruction {
                operation: Operation::Mov_Acc_To_Mem,
                operands: [ Some(dest_operand), Some(src_operand) ],
                flags,
            });
        },

        0b0000010 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = Operand::Register(0b000, RegisterAccess::new(0b000 /* register A */, wide));
            let src_operand = Operand::ImmediateData(data);
            let operands = [ Some(dest_operand), Some(src_operand) ];

            return Some(Instruction { operation: Operation::Add_Imm_To_Acc, operands, flags });
        },

        0b10110 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = Operand::Register(0b000, RegisterAccess::new(0b000 /* register A */, wide));
            let src_operand = Operand::ImmediateData(data);
            let operands = [ Some(dest_operand), Some(src_operand) ];

            return Some(Instruction { operation: Operation::Sub_Imm_From_Acc, operands, flags });
        },

        0b011110 => {
            let wide = byte & 0b1 == 1;
            let data = read_data(instruction_stream, wide);

            let mut flags: Vec<OperationFlag> = vec![];
            if wide { flags.push(OperationFlag::Wide); }

            let dest_operand = Operand::Register(0b000, RegisterAccess::new(0b000 /* register A */, wide));
            let src_operand = Operand::ImmediateData(data);
            let operands = [ Some(dest_operand), Some(src_operand) ];

            return Some(Instruction { operation: Operation::Cmp_Imm_With_Acc, operands, flags });
        },

        _ => {}
    };

    let opcode = byte;
    match opcode {
        0b01110100 => Some(Instruction { operation: Operation::Jmp_On_Equal, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111100 => Some(Instruction { operation: Operation::Jmp_On_Less, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111110 => Some(Instruction { operation: Operation::Jmp_On_Less_Or_Equal, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110010 => Some(Instruction { operation: Operation::Jmp_On_Below, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110110 => Some(Instruction { operation: Operation::Jmp_On_Below_Or_Equal, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111111 => Some(Instruction { operation: Operation::Jmp_On_Greater, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110111 => Some(Instruction { operation: Operation::Jmp_On_Above, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111010 => Some(Instruction { operation: Operation::Jmp_On_Parity, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110000 => Some(Instruction { operation: Operation::Jmp_On_Overflow, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111000 => Some(Instruction { operation: Operation::Jmp_On_Sign, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110101 => Some(Instruction { operation: Operation::Jmp_On_Not_Equal, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111101 => Some(Instruction { operation: Operation::Jmp_On_Not_Less, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110011 => Some(Instruction { operation: Operation::Jmp_On_Not_Below, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111011 => Some(Instruction { operation: Operation::Jmp_On_Not_Parity, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01110001 => Some(Instruction { operation: Operation::Jmp_On_Not_Overflow, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b01111001 => Some(Instruction { operation: Operation::Jmp_On_Not_Sign, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b11100011 => Some(Instruction { operation: Operation::Jmp_On_CX_Zero, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b11100010 => Some(Instruction { operation: Operation::Loop, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b11100001 => Some(Instruction { operation: Operation::Loop_While_Zero, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        0b11100000 => Some(Instruction { operation: Operation::Loop_While_Not_Zero, operands: [ Some(Operand::LabelOffset(read_byte(instruction_stream) as i8)), None ], flags: vec![] }),
        _ => None
    }
}
