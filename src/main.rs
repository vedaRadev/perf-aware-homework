use std::{
    env,
    process,
    io::prelude::*,
    fs::File,
};

mod decoder;
use decoder::*;

struct Flags { sign: bool, zero: bool }
impl Flags {
    fn new() -> Self { Self { sign: false, zero: false } }

    fn get_active_flags_string(&self) -> String {
        let mut str = String::with_capacity(2); // should match number of flag fields
        if self.sign { str += "S"; }
        if self.zero { str += "Z"; }

        str
    }
}

struct RegisterSet { registers: [u16; 8] }
impl RegisterSet {
    fn new() -> Self { Self { registers: [0u16; 8] } }

    fn get_register_value(&self, encoding: u8, access: &RegisterAccess) -> u16 {
        match access {
            RegisterAccess::Low => self.registers[encoding as usize].to_ne_bytes()[1] as u16,
            RegisterAccess::High => self.registers[encoding as usize].to_ne_bytes()[0] as u16,
            RegisterAccess::Full => self.registers[encoding as usize],
        }
    }

    fn set_register_value(&mut self, encoding: u8, access: &RegisterAccess, value: u16) {
        let register = &mut self.registers[encoding as usize];

        match access {
            RegisterAccess::Full => *register = value,
            RegisterAccess::High => set_high_byte(register, value as u8),
            RegisterAccess::Low => set_low_byte(register, value as u8),
        };
    }

    fn calculate_effective_address(&self, base: &EffectiveAddressBase, displacement: u16) -> u16 {
        base.get_register_encodings()
            .iter()
            .filter_map(|v| *v)
            .fold(displacement, |acc, reg| acc + self.get_register_value(reg, &RegisterAccess::Full))
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

    let mut file = File::open(&args[1]).unwrap_or_else(|_| panic!("Failed to open file {}", args[1]));
    let mut instruction_stream: Vec<u8> = vec![];
    file.read_to_end(&mut instruction_stream).expect("Failed to read file");

    // TODO move flags, instruction pointer into RegisterSet
    let mut register_set = RegisterSet::new();
    let mut flags = Flags::new();
    let mut instruction_pointer = 0;
    let mut memory = [0u8; u16::MAX as usize]; // 64k instead of 1MB since not using segment registers

    while instruction_pointer < instruction_stream.len() {
        let instruction = decode_instruction(&instruction_stream, instruction_pointer);
        if instruction.is_none() { break; }
        let instruction = instruction.unwrap();
        print!("{} ;", instruction);

        let flags_before = flags.get_active_flags_string();
        let instruction_pointer_before = instruction_pointer;
        instruction_pointer += instruction.size as usize;

        match &instruction.operands {
            [ Some(destination), Some(source) ] => {
                let source_value = match source {
                    Operand::Register(encoding, access) => register_set.get_register_value(*encoding, access),
                    Operand::ImmediateData(data) => *data,

                    Operand::Memory(EffectiveAddress::Direct(address)) => if instruction.flags.wide {
                        read_word(&memory, *address as usize)
                    } else {
                        memory[*address as usize] as u16
                    },

                    Operand::Memory(EffectiveAddress::Calculated { base, displacement }) => {
                        let address = register_set.calculate_effective_address(base, *displacement);

                        if instruction.flags.wide {
                            read_word(&memory, address as usize)
                        } else {
                            memory[address as usize] as u16
                        }
                    },

                    Operand::LabelOffset(_) => panic!("offset value cannot be a source"),
                };

                let mut destination_value_before: Option<u16> = None;
                let mut destination_value_after: Option<u16> = None;

                match instruction.operation {
                    Operation::Mov_RegMem_ToFrom_Reg
                    | Operation::Mov_Imm_To_Reg
                    | Operation::Mov_Imm_To_RegMem
                    | Operation::Mov_Mem_To_Acc
                    | Operation::Mov_Acc_To_Mem => {
                        match destination {
                            Operand::Register(encoding, access) => {
                                destination_value_before = Some(register_set.get_register_value(*encoding, &RegisterAccess::Full));
                                register_set.set_register_value(*encoding, access, source_value);
                                destination_value_after = Some(register_set.get_register_value(*encoding, &RegisterAccess::Full));
                            },

                            Operand::Memory(EffectiveAddress::Direct(address)) => {
                                let address = *address as usize;
                                let [ hi, lo ] = source_value.to_ne_bytes();
                                if instruction.flags.wide {
                                    memory[address] = hi;
                                    memory[address + 1] = lo;
                                } else {
                                    memory[address] = lo;
                                }
                            },

                            Operand::Memory(EffectiveAddress::Calculated { base, displacement }) => {
                                let address = register_set.calculate_effective_address(base, *displacement) as usize;
                                let [ hi, lo ] = source_value.to_ne_bytes();
                                if instruction.flags.wide {
                                    memory[address] = hi;
                                    memory[address + 1] = lo;
                                } else {
                                    memory[address] = lo;
                                }
                            },

                            _ => panic!("cannot move into immediate or label offset"),
                        };
                    },

                    Operation::Add_RegMem_With_Reg_To_Either
                    | Operation::Add_Imm_To_RegMem
                    | Operation::Add_Imm_To_Acc => {
                        match destination {
                            Operand::Register(encoding, access) => {
                                let reg_val = register_set.get_register_value(*encoding, &RegisterAccess::Full);
                                destination_value_before = Some(reg_val);

                                register_set.set_register_value(*encoding, access, reg_val + source_value);
                                let reg_val_after = register_set.get_register_value(*encoding, access);
                                flags.zero = reg_val_after == 0;
                                flags.sign = match access {
                                    RegisterAccess::Full => (reg_val_after as i16) < 0,
                                    RegisterAccess::Low => (reg_val_after.to_ne_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (reg_val_after.to_ne_bytes()[0] as i8) < 0,
                                };

                                destination_value_after = Some(register_set.get_register_value(*encoding, &RegisterAccess::Full));
                            },

                            Operand::Memory(_) => todo!(),

                            _ => panic!("cannot add into immediate or label offset"),
                        }
                    },

                    Operation::Sub_RegMem_And_Reg_From_Either
                    | Operation::Sub_Imm_From_RegMem
                    | Operation::Sub_Imm_From_Acc => {
                        match destination {
                            Operand::Register(encoding, access) => {
                                let reg_val = register_set.get_register_value(*encoding, &RegisterAccess::Full);
                                destination_value_before = Some(reg_val);

                                register_set.set_register_value(*encoding, access, reg_val - source_value);
                                let reg_val_after = register_set.get_register_value(*encoding, access);
                                flags.zero = reg_val_after == 0;
                                flags.sign = match access {
                                    RegisterAccess::Full => (reg_val_after as i16) < 0,
                                    RegisterAccess::Low => (reg_val_after.to_ne_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (reg_val_after.to_ne_bytes()[0] as i8) < 0,
                                };

                                destination_value_after = Some(register_set.get_register_value(*encoding, &RegisterAccess::Full));
                            },

                            Operand::Memory(_) => todo!(),

                            _ => panic!("cannot sub from immediate or label offset"),
                        }
                    },

                    Operation::Cmp_RegMem_And_Reg
                    | Operation::Cmp_Imm_With_RegMem
                    | Operation::Cmp_Imm_With_Acc => {
                        match destination {
                            Operand::Register(encoding, access) => {
                                let reg_val = register_set.get_register_value(*encoding, &RegisterAccess::Full);
                                destination_value_before = Some(reg_val);

                                let test_val = ((reg_val as i16) - (source_value as i16)) as u16;
                                flags.zero = test_val == 0;
                                flags.sign = match access {
                                    RegisterAccess::Full => (test_val as i16) < 0,
                                    RegisterAccess::Low => (test_val.to_ne_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (test_val.to_ne_bytes()[0] as i8) < 0,
                                };

                                destination_value_after = Some(register_set.get_register_value(*encoding, &RegisterAccess::Full));
                            },

                            Operand::Memory(_) => todo!(),

                            _ => panic!("cannot cmp immediate or label offset"),
                        }
                    },

                    _ => panic!("Invalid 2-operand instruction encountered")
                };


                if destination_value_before.is_some() && destination_value_after.is_some() {
                    print!(" {}:{:#x}->{:#x}", destination, destination_value_before.unwrap(), destination_value_after.unwrap());
                }
            },

            [ Some(Operand::LabelOffset(offset)), None ] => {
                match instruction.operation {
                    Operation::Jmp_On_Not_Equal => if !flags.zero { instruction_pointer = ((instruction_pointer as isize) + *offset as isize) as usize },

                    _ => todo!("this conditional jump not implemented")
                };
            },
            [ Some(_), None ] => todo!("single-operand non-label-offset encountered"),
            [ None, None ] => todo!("0-operand instructions not implemented"),
            _ => panic!("invalid operand configuration [ None, Some(...) ]"),
        };

        print!(" ip:{:#x}->{:#x}", instruction_pointer_before, instruction_pointer);
        let flags_after = flags.get_active_flags_string();
        if flags_after.len() != flags_before.len() { print!(" flags:{}->{}", flags_before, flags_after); }
        println!();
    }

    println!("\nFinal register states:");
    for (register_index, value) in register_set.registers.iter().enumerate() {
        println!("\t{}: {:#06x} ({})", get_register_name(register_index as u8, true).expect("Invalid register"), value, value);
    }
    println!();
    println!("ip: {:#x} ({})", instruction_pointer, instruction_pointer);
    println!("flags: {}", flags.get_active_flags_string());
}
