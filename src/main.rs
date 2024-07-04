use std::{
    env,
    process,
    io::prelude::*,
    fs,
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
            RegisterAccess::Low => self.registers[encoding as usize].to_be_bytes()[1] as u16,
            RegisterAccess::High => self.registers[encoding as usize].to_be_bytes()[0] as u16,
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

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut assembly_filename: Option<&str> = None;
    let mut memdump_filename: Option<&str> = None;
    let mut should_execute = false;
    let mut should_dump_memory = false;
    // we'll skip the first arg since it should just be the executable filename
    let mut arg_index = 1;
    // probably dumb way to parse args
    loop {
        if arg_index >= args.len() { break; }
        match args[arg_index].as_str() {
            "--execute" => {
                should_execute = true;
                arg_index += 1;
            },

            "--memdump" => {
                should_dump_memory = true;
                if args[arg_index + 1].starts_with('-') {
                    println!("memdump arg requires a filename to dump to");
                    process::exit(1);
                }

                memdump_filename = Some(&args[arg_index + 1]);

                arg_index += 2;
            },

            _ => {
                if assembly_filename.is_some() {
                    println!("more than one input file specified, aborting");
                    process::exit(1);
                }

                if args[arg_index].starts_with('-') {
                    println!("unrecognized command line argument");
                    process::exit(1);
                }

                assembly_filename = Some(&args[arg_index]);

                arg_index += 1;
            }
        }
    }

    let assembly_filename = assembly_filename.unwrap_or_else(|| {
        println!("assembled binary not supplied, aborting");
        process::exit(1);
    });
    let memdump_filename = memdump_filename.unwrap_or("");

    if should_dump_memory && !should_execute {
        println!("Memory can only be dumped if executing a program");
        process::exit(1);
    }

    let mut file = fs::File::open(assembly_filename).unwrap_or_else(|_| panic!("Failed to open file {}", assembly_filename));
    let mut instruction_stream: Vec<u8> = vec![];
    file.read_to_end(&mut instruction_stream).expect("Failed to read file");

    // TODO move flags, instruction pointer into RegisterSet
    let mut register_set = RegisterSet::new();
    let mut flags = Flags::new();
    let mut instruction_pointer = 0;
    let mut memory = [0u8; u16::MAX as usize]; // 64k instead of 1MB since not using segment registers
    memory[..instruction_stream.len()].copy_from_slice(&instruction_stream);
    memory[instruction_stream.len()] = 0b11110100; // insert HALT at end of instructions
    drop(instruction_stream);

    loop {
        let instruction = decode_instruction(&memory, instruction_pointer);
        if instruction.is_none() {
            println!("illegal or unimplemented operation encountered, halting");
            break;
        }

        let instruction = instruction.unwrap();
        if instruction.operation == Operation::Halt { break; }

        instruction_pointer += instruction.size as usize;

        if !should_execute {
            println!("{}", instruction);
            continue;
        }

        print!("{} ;", instruction);

        let flags_before = flags.get_active_flags_string();
        let instruction_pointer_before = instruction_pointer;

        match &instruction.operands {
            [ Some(destination), Some(source) ] => {
                let source_value: u16 = match source {
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
                                let [ hi, lo ] = source_value.to_be_bytes();
                                if instruction.flags.wide {
                                    memory[address] = hi;
                                    memory[address + 1] = lo;
                                } else {
                                    memory[address] = lo;
                                }
                            },

                            Operand::Memory(EffectiveAddress::Calculated { base, displacement }) => {
                                let address = register_set.calculate_effective_address(base, *displacement) as usize;
                                let [ hi, lo ] = source_value.to_be_bytes();
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
                                    RegisterAccess::Low => (reg_val_after.to_be_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (reg_val_after.to_be_bytes()[0] as i8) < 0,
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
                                    RegisterAccess::Low => (reg_val_after.to_be_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (reg_val_after.to_be_bytes()[0] as i8) < 0,
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
                                    RegisterAccess::Low => (test_val.to_be_bytes()[1] as i8) < 0,
                                    RegisterAccess::High => (test_val.to_be_bytes()[0] as i8) < 0,
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
                    // TODO stop hardcoding register indices
                    Operation::Loop => {
                        let cx_value = register_set.get_register_value(1 /* C */, &RegisterAccess::Full);
                        register_set.set_register_value(1 /* C */, &RegisterAccess::Full, cx_value - 1);
                        let cx_value = register_set.get_register_value(1 /* C */, &RegisterAccess::Full);
                        if cx_value != 0 {
                            instruction_pointer = ((instruction_pointer as isize) + *offset as isize) as usize;
                        }
                    },

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

    if should_execute {
        println!("\nFinal register states:");
        for (register_index, value) in register_set.registers.iter().enumerate() {
            println!("\t{}: {:#06x} ({})", get_register_name(register_index as u8, true).expect("Invalid register"), value, value);
        }
        println!();
        println!("ip: {:#x} ({})", instruction_pointer, instruction_pointer);
        println!("flags: {}", flags.get_active_flags_string());

        if should_dump_memory {
            fs::write(memdump_filename, memory).expect("Failed to write memdump to file");
            println!();
            println!("memory dumped to {}", memdump_filename);
        }
    }
}
