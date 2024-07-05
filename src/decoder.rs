use std::fmt;

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

impl EffectiveAddressBase {
    pub fn get_register_encodings(&self) -> [Option<u8>; 2] {
        // FIXME hardcoded trash, find a better way
        match self {
            Self::BX_SI => [ Some(3), Some(6) ],
            Self::BX_DI => [ Some(3), Some(7) ],
            Self::BP_SI => [ Some(5), Some(6) ],
            Self::BP_DI => [ Some(5), Some(7) ],
            Self::SI => [ Some(6), None ],
            Self::DI => [ Some(7), None ],
            Self::BP => [ Some(5), None ],
            Self::BX => [ Some(3), None ],
        }
    }
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

    fn get_clocks_estimate(&self) -> u16 {
        // if implemented, add 2 clocks when a segment override is present
        match self {
            Self::Direct(_) => 6,
            // TODO need to add 4 clocks for each 16-bit word transfer w/ odd address (see manual)
            Self::Calculated { base, displacement } => match base {
                EffectiveAddressBase::BX
                | EffectiveAddressBase::BP
                | EffectiveAddressBase::SI
                | EffectiveAddressBase::DI
                => if *displacement == 0 { 5 } else { 9 },

                EffectiveAddressBase::BP_DI
                | EffectiveAddressBase::BX_SI
                => if *displacement == 0 { 7 } else { 11 },

                EffectiveAddressBase::BP_SI
                | EffectiveAddressBase::BX_DI
                => if *displacement == 0 { 8 } else { 12 }
            }
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

                let [disp_hi, disp_lo] = displacement.to_be_bytes();
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

impl Operand {
    fn register_or_memory(mode: u8, reg_or_mem: u8, displacement: u16, wide: bool) -> Self {
        if mode == 0b11 {
            Self::Register(reg_or_mem, RegisterAccess::new(reg_or_mem, wide))
        } else {
            Self::Memory(EffectiveAddress::new(mode, reg_or_mem, displacement))
        }
    }

    fn register_acc(wide: bool) -> Self { Self::Register(0b000, RegisterAccess::new(0b000, wide)) }
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

#[allow(dead_code)]
#[derive(Default)]
pub struct InstructionFlags {
    pub sign_extend: bool,
    pub wide: bool,
    pub destination: bool,
    pub v: bool, // false - shift/rotate count is 1, true - specified in CL reg
    pub repeat_on_zero: bool,
}

#[allow(non_camel_case_types)]
#[derive(PartialEq)]
pub enum Operation {
    Mov_RegMem_ToFrom_Reg,
    Mov_Imm_To_RegMem,
    Mov_Imm_To_Reg,
    Mov_Mem_To_Acc,
    Mov_Acc_To_Mem,

    Add_RegMem_With_Reg_To_Either,
    Add_Imm_To_RegMem,
    Add_Imm_To_Acc,

    Sub_RegMem_And_Reg_From_Either,
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
    
    Halt, // hlt
}

pub struct Instruction {
    pub operation: Operation,
    pub operands: [Option<Operand>; 2], // e.g. opcode operand_1, operand_2 (max 2 operands)
    pub flags: InstructionFlags,
    pub size: u8,
}

type ClockEstimate = u16;
type ClockExplanation = String;
fn get_ea_clocks_and_explanation(base_clocks: u16, ea: &EffectiveAddress) -> (ClockEstimate, Option<ClockExplanation>) {
    let ea_clocks = ea.get_clocks_estimate();
    let clocks = base_clocks + ea_clocks;
    (clocks, Some(format!("{} + {}ea", base_clocks, ea_clocks)))
}

impl Instruction {
    pub fn get_clocks_estimate(&self) -> (ClockEstimate, Option<ClockExplanation>) {
        match self.operation {
            Operation::Mov_Acc_To_Mem | Operation::Mov_Mem_To_Acc => (10, None),
            Operation::Mov_Imm_To_Reg => (4, None),
            Operation::Mov_Imm_To_RegMem => match &self.operands[0] {
                Some(Operand::Register(..)) => (4, None),
                Some(Operand::Memory(ea)) => get_ea_clocks_and_explanation(10, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },
            Operation::Mov_RegMem_ToFrom_Reg => match &self.operands {
                [ Some(Operand::Register(..)), Some(Operand::Register(..)) ] => (2, None),
                [ Some(Operand::Register(..)), Some(Operand::Memory(ea)) ] => get_ea_clocks_and_explanation(8, ea),
                [ Some(Operand::Memory(ea)), Some(Operand::Register(..)) ] => get_ea_clocks_and_explanation(9, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },

            Operation::Add_Imm_To_Acc => (4, None),
            Operation::Add_Imm_To_RegMem => {
                let [ dst_op, _ ] = &self.operands;
                match dst_op {
                    Some(Operand::Register(..)) => (4, None),
                    Some(Operand::Memory(ea)) => get_ea_clocks_and_explanation(17, ea),
                    _ => panic!("other ops not supported for this operation. how did this happen?")
                }
            },
            Operation::Add_RegMem_With_Reg_To_Either => {
                match &self.operands {
                    [ Some(Operand::Register(..)), Some(Operand::Register(..)) ] => (3, None),
                    [ Some(Operand::Register(..)), Some(Operand::Memory(ea)) ] => get_ea_clocks_and_explanation(9, ea),
                    [ Some(Operand::Memory(ea)), Some(Operand::Register(..)) ] => get_ea_clocks_and_explanation(16, ea),
                    _ => panic!("other operand combos not supported for this operation")
                }
            },

            Operation::Sub_Imm_From_Acc => (4, None),
            Operation::Sub_Imm_From_RegMem => match &self.operands[0] {
                Some(Operand::Register(..)) => (4, None),
                Some(Operand::Memory(ea)) => get_ea_clocks_and_explanation(17, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },
            Operation::Sub_RegMem_And_Reg_From_Either => match &self.operands {
                [ Some(Operand::Register(..)), Some(Operand::Register(..)) ] => (3, None),
                [ Some(Operand::Register(..)), Some(Operand::Memory(ea)) ] => get_ea_clocks_and_explanation(9, ea),
                [ Some(Operand::Memory(ea)), Some(Operand::Register(..)) ] => get_ea_clocks_and_explanation(16, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },

            Operation::Cmp_Imm_With_Acc => (4, None),
            Operation::Cmp_Imm_With_RegMem => match &self.operands[0] {
                Some(Operand::Register(..)) => (4, None),
                Some(Operand::Memory(ea)) => get_ea_clocks_and_explanation(10, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },
            Operation::Cmp_RegMem_And_Reg => match &self.operands {
                [ Some(Operand::Register(..)), Some(Operand::Register(..)) ] => (3, None),
                [ Some(Operand::Register(..)), Some(Operand::Memory(ea)) ] => get_ea_clocks_and_explanation(9, ea),
                [ Some(Operand::Memory(ea)), Some(Operand::Register(..)) ] => get_ea_clocks_and_explanation(9, ea),
                _ => panic!("other operand combos not supported for this operation"),
            },


            Operation::Jmp_On_Equal
            | Operation::Jmp_On_Less
            | Operation::Jmp_On_Less_Or_Equal
            | Operation::Jmp_On_Below
            | Operation::Jmp_On_Below_Or_Equal
            | Operation::Jmp_On_Greater
            | Operation::Jmp_On_Above
            | Operation::Jmp_On_Parity
            | Operation::Jmp_On_Overflow
            | Operation::Jmp_On_Sign
            | Operation::Jmp_On_Not_Equal
            | Operation::Jmp_On_Not_Less
            | Operation::Jmp_On_Not_Below
            | Operation::Jmp_On_Not_Parity
            | Operation::Jmp_On_Not_Overflow
            | Operation::Jmp_On_Not_Sign
            | Operation::Jmp_On_CX_Zero
            => (4, None),

            Operation::Loop => (5, None),
            Operation::Loop_While_Zero => (6, None),
            Operation::Loop_While_Not_Zero => (5, None),
            
            Operation::Halt => (2, None),
        }
    }
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

            Operation::Add_RegMem_With_Reg_To_Either
            | Operation::Add_Imm_To_RegMem
            | Operation::Add_Imm_To_Acc
                => "add",

            Operation::Sub_RegMem_And_Reg_From_Either
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

            Operation::Halt => "hlt",
        };

        match &self.operands {
            [ None, None ] => todo!("printing for 0-operand instructions not implemented"),
            [ Some(operand), None ] => write!(formatter, "{} {}", op_name, operand),
            [ Some(dst @ Operand::Memory(_)), Some(src @ Operand::ImmediateData(_)) ] => {
                let size_specifier = if self.flags.wide { "word" } else { "byte" };
                write!(formatter, "{} {}, {} {}", op_name, dst, size_specifier, src)
            },
            [ Some(dst), Some(src) ] => write!(formatter, "{} {}, {}", op_name, dst, src),
            _ => panic!("invalid operand configuration [None, Some(...)]")
        }
    }
}

// TODO move to utils file
pub fn read_word(bytes: &[u8], at: usize) -> u16 {
    let lo = bytes[at];
    let hi = bytes[at + 1];
    (hi as u16) << 8 | lo as u16
}

fn read_displacement(instruction_stream: &[u8], displacement_index: usize, mode: u8, reg_or_mem: u8) -> (u16, u8) {
    if mode == 0b10 || mode == 0b00 && reg_or_mem == 0b110 {
        (read_word(instruction_stream, displacement_index), 2)
    } else if mode == 0b01 {
        (instruction_stream[displacement_index] as u16, 1)
    } else {
        (0, 0)
    }
}

fn read_data(instruction_stream: &[u8], data_index: usize, is_word: bool) -> (u16, u8) {
    if is_word {
        (read_word(instruction_stream, data_index), 2)
    } else {
        (instruction_stream[data_index] as u16, 1)
    }
}

pub fn decode_instruction(instruction_stream: &[u8], instruction_pointer: usize) -> Option<Instruction> {
    let maybe_opcode = instruction_stream[instruction_pointer];
    let operation: Option<Operation> = match maybe_opcode >> 4 {
        0b1011 => Some(Operation::Mov_Imm_To_Reg),
        _ => match maybe_opcode >> 2 {
            0b100010 => Some(Operation::Mov_RegMem_ToFrom_Reg),
            0b000000 => Some(Operation::Add_RegMem_With_Reg_To_Either),
            0b001010 => Some(Operation::Sub_RegMem_And_Reg_From_Either),
            0b001110 => Some(Operation::Cmp_RegMem_And_Reg),
            0b100000 => match (instruction_stream[instruction_pointer + 1] & 0b111000) >> 3 {
                0b000 => Some(Operation::Add_Imm_To_RegMem),
                0b101 => Some(Operation::Sub_Imm_From_RegMem),
                0b111 => Some(Operation::Cmp_Imm_With_RegMem),
                _ => None
            },
            _ => match maybe_opcode >> 1 {
                0b1100011 => Some(Operation::Mov_Imm_To_RegMem),
                0b1010000 => Some(Operation::Mov_Mem_To_Acc),
                0b1010001 => Some(Operation::Mov_Acc_To_Mem),
                0b0000010 => Some(Operation::Add_Imm_To_Acc),
                0b0010110 => Some(Operation::Sub_Imm_From_Acc),
                0b0011110 => Some(Operation::Cmp_Imm_With_Acc),
                _ => match maybe_opcode {
                    0b01110100 => Some(Operation::Jmp_On_Equal),
                    0b01111100 => Some(Operation::Jmp_On_Less),
                    0b01111110 => Some(Operation::Jmp_On_Less_Or_Equal),
                    0b01110010 => Some(Operation::Jmp_On_Below),
                    0b01110110 => Some(Operation::Jmp_On_Below_Or_Equal),
                    0b01111111 => Some(Operation::Jmp_On_Greater),
                    0b01110111 => Some(Operation::Jmp_On_Above),
                    0b01111010 => Some(Operation::Jmp_On_Parity),
                    0b01110000 => Some(Operation::Jmp_On_Overflow),
                    0b01111000 => Some(Operation::Jmp_On_Sign),
                    0b01110101 => Some(Operation::Jmp_On_Not_Equal),
                    0b01111101 => Some(Operation::Jmp_On_Not_Less),
                    0b01110011 => Some(Operation::Jmp_On_Not_Below),
                    0b01111011 => Some(Operation::Jmp_On_Not_Parity),
                    0b01110001 => Some(Operation::Jmp_On_Not_Overflow),
                    0b01111001 => Some(Operation::Jmp_On_Not_Sign),
                    0b11100011 => Some(Operation::Jmp_On_CX_Zero),
                    0b11100010 => Some(Operation::Loop),
                    0b11100001 => Some(Operation::Loop_While_Zero),
                    0b11100000 => Some(Operation::Loop_While_Not_Zero),

                    0b11110100 => Some(Operation::Halt),

                    _ => None
                }
            }
        }
    };

    #[allow(clippy::question_mark)]
    if operation.is_none() { return None; }
    let operation = operation.unwrap();

    match operation {
        Operation::Mov_Imm_To_Reg => {
            const BASE_INSTRUCTION_LENGTH: u8 = 1;

            let params = instruction_stream[instruction_pointer];

            let wide = (params >> 3) & 1 == 1;
            let reg = params & 0b111;
            let (data, data_length) = read_data(instruction_stream, instruction_pointer + 1, wide);

            let destination_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let source_operand = Operand::ImmediateData(data);
            let operands = [ Some(destination_operand), Some(source_operand) ];
            let flags = InstructionFlags { wide, ..Default::default() };

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH + data_length })
        },

        Operation::Mov_RegMem_ToFrom_Reg
        | Operation::Add_RegMem_With_Reg_To_Either
        | Operation::Sub_RegMem_And_Reg_From_Either
        | Operation::Cmp_RegMem_And_Reg
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 2;

            let params = instruction_stream[instruction_pointer];
            let operands = instruction_stream[instruction_pointer + 1];

            let dest = params & 0b10 > 0;
            let wide = params & 1 == 1;
            let mode = operands >> 6;
            let reg = (operands & 0b111000) >> 3;
            let reg_or_mem = operands & 0b111;
            let (displacement, displacement_length) = read_displacement(
                instruction_stream,
                instruction_pointer + 2,
                mode,
                reg_or_mem
            );

            let flags = InstructionFlags { wide, destination: dest, ..Default::default() };

            let mut destination_operand = Operand::Register(reg, RegisterAccess::new(reg, wide));
            let mut source_operand = Operand::register_or_memory(mode, reg_or_mem, displacement, wide);

            if !dest { std::mem::swap(&mut destination_operand, &mut source_operand); }
            let operands = [ Some(destination_operand), Some(source_operand) ];

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH + displacement_length })
        },

        Operation::Add_Imm_To_RegMem
        | Operation::Sub_Imm_From_RegMem
        | Operation::Cmp_Imm_With_RegMem
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 2;

            let params = instruction_stream[instruction_pointer];
            let operands = instruction_stream[instruction_pointer + 1];

            let sign_extend = params & 0b10 > 0;
            let wide = params & 1 == 1;
            let mode = operands >> 6;
            let reg_or_mem = operands & 0b111;
            let (displacement, displacement_length) = read_displacement(
                instruction_stream,
                instruction_pointer + 2,
                mode,
                reg_or_mem
            );
            let (data, data_length) = read_data(
                instruction_stream,
                instruction_pointer + 2 + displacement_length as usize,
                !sign_extend && wide
            );

            let flags = InstructionFlags { sign_extend, wide, ..Default::default() };

            let destination_operand = Operand::register_or_memory(mode, reg_or_mem, displacement, wide);
            let source_operand = Operand::ImmediateData(data);
            let operands = [ Some(destination_operand), Some(source_operand) ];

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH + displacement_length + data_length })
        },

        Operation::Mov_Imm_To_RegMem => {
            const BASE_INSTRUCTION_LENGTH: u8 = 2;

            let params = instruction_stream[instruction_pointer];
            let operands = instruction_stream[instruction_pointer + 1];

            let wide = params & 1 == 1;
            let mode = operands >> 6;
            let reg_or_mem = operands & 0b111;
            let (displacement, displacement_length) = read_displacement(
                instruction_stream,
                instruction_pointer + 2,
                mode,
                reg_or_mem
            );
            let (data, data_length) = read_data(
                instruction_stream,
                instruction_pointer + 2 + displacement_length as usize,
                wide
            );

            let flags = InstructionFlags { wide, ..Default::default() };

            let destination_operand = Operand::register_or_memory(mode, reg_or_mem, displacement, wide);
            let source_operand = Operand::ImmediateData(data);
            let operands = [ Some(destination_operand), Some(source_operand) ];

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH + displacement_length + data_length })
        },

        Operation::Mov_Mem_To_Acc
        | Operation::Mov_Acc_To_Mem
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 3;

            let params = instruction_stream[instruction_pointer];
            let wide = params & 1 == 1;
            let address = read_word(instruction_stream, instruction_pointer + 1);

            // assuming mem -> acc
            let mut destination_operand = Operand::register_acc(wide);
            let mut source_operand = Operand::Memory(EffectiveAddress::Direct(address));
            // swap destination and source if acc -> mem
            if operation == Operation::Mov_Acc_To_Mem { std::mem::swap(&mut destination_operand, &mut source_operand); }
            let operands = [ Some(destination_operand), Some(source_operand) ];

            let flags = InstructionFlags { wide, ..Default::default() };

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH })
        },

        Operation::Add_Imm_To_Acc
        | Operation::Sub_Imm_From_Acc
        | Operation::Cmp_Imm_With_Acc
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 1;

            let params = instruction_stream[instruction_pointer];
            let wide = params & 1 == 1;
            let (data, data_length) = read_data(instruction_stream, instruction_pointer + 1, wide);

            let destination_operand = Operand::register_acc(wide);
            let source_operand = Operand::ImmediateData(data);
            let operands = [ Some(destination_operand), Some(source_operand) ];

            let flags = InstructionFlags { wide, ..Default::default() };

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH + data_length })
        },

        Operation::Jmp_On_Equal
        | Operation::Jmp_On_Less
        | Operation::Jmp_On_Less_Or_Equal
        | Operation::Jmp_On_Below
        | Operation::Jmp_On_Below_Or_Equal
        | Operation::Jmp_On_Greater
        | Operation::Jmp_On_Above
        | Operation::Jmp_On_Parity
        | Operation::Jmp_On_Overflow
        | Operation::Jmp_On_Sign
        | Operation::Jmp_On_Not_Equal
        | Operation::Jmp_On_Not_Less
        | Operation::Jmp_On_Not_Below
        | Operation::Jmp_On_Not_Parity
        | Operation::Jmp_On_Not_Overflow
        | Operation::Jmp_On_Not_Sign
        | Operation::Jmp_On_CX_Zero
        | Operation::Loop
        | Operation::Loop_While_Zero
        | Operation::Loop_While_Not_Zero
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 2;

            let operands = [ Some(Operand::LabelOffset(instruction_stream[instruction_pointer + 1] as i8)), None ];
            let flags = InstructionFlags::default();

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH })
        }

        Operation::Halt
        => {
            const BASE_INSTRUCTION_LENGTH: u8 = 1;
            let operands = [ None, None ];
            let flags = InstructionFlags::default();

            Some(Instruction { operation, operands, flags, size: BASE_INSTRUCTION_LENGTH })
        },
    }
}
