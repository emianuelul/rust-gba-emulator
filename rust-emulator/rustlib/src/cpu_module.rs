use crate::memory_area::GBAMemory;
use bitmatch::bitmatch;
use std::collections::HashMap;
use tracing::{error, warn};

// fetch
//   V
// decode
//   V
// execute

// TODO: refactor get_register_value and set_register_value to work for THUMB

const SP: usize = 13;
const LR: usize = 14;
const PC: usize = 15;

const N_FLAG: usize = 31;
const Z_FLAG: usize = 30;
const C_FLAG: usize = 29;
const V_FLAG: usize = 28;

const IRQ_FLAG: usize = 7;
const FIQ_FLAG: usize = 6;
const T_FLAG: usize = 5;

#[derive(Hash, Eq, PartialEq)]
enum CPUMode {
    UserSys,
    User,
    Sys,
    Fiq,
    Supervisor,
    Abort,
    Irq,
    Undefined,
}

enum CPUState {
    Arm,
    Thumb,
}

// pointers: r13, r14 (SP, LR)
struct CPURegisters {
    common: Vec<u32>, // r0-r12
    fiq: Vec<u32>,    // r8-r12

    pointers: HashMap<CPUMode, (u32, u32)>,
    pc: u32,
}

impl CPURegisters {
    fn new() -> Self {
        CPURegisters {
            common: [0; 13].to_vec(),
            fiq: [0; 5].to_vec(),
            pointers: HashMap::from([
                (CPUMode::UserSys, (0, 0)),
                (CPUMode::Fiq, (0, 0)),
                (CPUMode::Supervisor, (0, 0)),
                (CPUMode::Abort, (0, 0)),
                (CPUMode::Irq, (0, 0)),
                (CPUMode::Undefined, (0, 0)),
            ]),
            pc: 0,
        }
    }
}

pub struct CPU {
    registers: CPURegisters,
    cpsr: u32,
    spsr: HashMap<CPUMode, u32>,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            registers: CPURegisters::new(),
            cpsr: 0x0000_00D3,
            spsr: HashMap::from([
                (CPUMode::Fiq, 0),
                (CPUMode::Supervisor, 0),
                (CPUMode::Abort, 0),
                (CPUMode::Irq, 0),
                (CPUMode::Undefined, 0),
            ]),
        }
    }
}

impl Default for CPU {
    fn default() -> Self {
        CPU::new()
    }
}

// Registers I/O
impl CPU {
    fn set_register_value(&mut self, index: usize, data: u32) {
        let curr_mode = self.get_effective_cpu_mode();
        match index {
            0..8 => self.registers.common[index] = data,
            8..=12 => {
                if curr_mode == CPUMode::Fiq {
                    self.registers.fiq[index - 8] = data;
                } else {
                    self.registers.common[index] = data;
                }
            }
            13..=14 => {
                let value = self.registers.pointers.get(&curr_mode).unwrap();

                if index == 13 {
                    self.registers.pointers.insert(curr_mode, (data, value.1));
                } else {
                    self.registers.pointers.insert(curr_mode, (value.0, data));
                }
            }
            PC => self.registers.pc = data,

            _ => {
                error!("CPU Register read index out of bounds: {}", index);
            }
        }
    }

    fn get_register_value(&self, index: usize) -> u32 {
        let curr_mode = self.get_effective_cpu_mode();
        match index {
            0..8 => self.registers.common[index],
            8..=12 => {
                if curr_mode == CPUMode::Fiq {
                    self.registers.fiq[index - 8]
                } else {
                    self.registers.common[index]
                }
            }
            13..=14 => {
                let value = self.registers.pointers.get(&curr_mode).unwrap();

                if index == 13 {
                    value.0
                } else {
                    value.1
                }
            }
            PC => self.registers.pc,

            _ => {
                error!("CPU Register read index out of bounds: {}", index);
                0
            }
        }
    }
}

// CPSR Ops
impl CPU {
    fn get_cpsr_bit(&self, index: usize) -> u8 {
        ((self.cpsr >> index) & 1) as u8
    }

    fn set_cpsr_bit(&mut self, index: usize, value: bool) {
        if value {
            self.cpsr |= 1 << index
        } else {
            self.cpsr &= !(1 << index)
        }
    }

    fn get_cpu_state(&self) -> CPUState {
        if self.get_cpsr_bit(T_FLAG) == 0 {
            CPUState::Arm
        } else {
            CPUState::Thumb
        }
    }

    fn get_cpu_mode(&self) -> CPUMode {
        let m40: u8 = (self.get_cpsr_bit(4) << 4)
            | (self.get_cpsr_bit(3) << 3)
            | (self.get_cpsr_bit(2) << 2)
            | (self.get_cpsr_bit(1) << 1)
            | (self.get_cpsr_bit(0));

        match m40 {
            0b10000 => CPUMode::User,
            0b10001 => CPUMode::Fiq,
            0b10010 => CPUMode::Irq,
            0b10011 => CPUMode::Supervisor,
            0b10111 => CPUMode::Abort,
            0b11011 => CPUMode::Undefined,
            0b11111 => CPUMode::Sys,
            _ => {
                error!("Mode bits are set in invalid formation: {:x?}", m40);
                CPUMode::Undefined
            }
        }
    }

    fn get_effective_cpu_mode(&self) -> CPUMode {
        if [CPUMode::User, CPUMode::Sys].contains(&self.get_cpu_mode()) {
            CPUMode::UserSys
        } else {
            self.get_cpu_mode()
        }
    }

    fn check_condition(&self, cond_bits: u8) -> bool {
        let n = self.get_cpsr_bit(N_FLAG);
        let z = self.get_cpsr_bit(Z_FLAG);
        let c = self.get_cpsr_bit(C_FLAG);
        let v = self.get_cpsr_bit(V_FLAG);

        match cond_bits {
            // EQ
            0x0 => z == 1,

            // NE
            0x1 => z == 0,

            // CS/HS
            0x2 => c == 1,

            // CC/LO
            0x3 => c == 0,

            // MI
            0x4 => n == 1,

            // PL
            0x5 => n == 0,

            // VS
            0x6 => v == 1,

            // VC
            0x7 => v == 0,

            // HI
            0x8 => c == 1 && z == 0,

            // LS
            0x9 => c == 0 || z == 1,

            // GE
            0xA => n == v,

            // LT
            0xB => n != v,

            // GT
            0xC => z == 0 && n == v,

            // LE
            0xD => z == 1 || n != v,

            // AL
            0xE => true,

            // NV
            _ => {
                warn!("Never condition code found");
                false
            }
        }
    }
}

// Step Logic
// TODO: Revisit after waitcnt
impl CPU {
    fn convert_u24_to_i32(&self, value: u32) -> i32 {
        ((value << 8) as i32) >> 8
    }

    fn execute_alu_op(&mut self, opcode: u8, rn: u8, rd: u8, operand2: u32, s: bool) {}

    #[bitmatch]
    pub fn step(&mut self, memory: &GBAMemory) -> u32 {
        if self.registers.pc >= memory.get_rom_size() as u32 {
            error!(
                "PC tried to go over allowed memory limit {:x?}",
                self.registers.pc
            );
            return 0;
        }

        let mut clk: u32 = 0;

        match self.get_cpu_state() {
            CPUState::Arm => {
                // fetch
                let (instruction, read) = memory.read32(self.get_register_value(PC));
                clk += read;

                #[bitmatch]
                let "cccc_????????????????????????????" = instruction;

                if self.check_condition(c as u8) {
                    #[bitmatch]
                    match instruction {
                        // B
                        "????_101_0_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            self.registers.pc =
                                (self.registers.pc as i32 + 8 + self.convert_u24_to_i32(n) * 4)
                                    as u32;

                            // 2S + 1N
                        }

                        // BL
                        "????_101_1_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            self.set_register_value(LR, self.registers.pc + 4);
                            self.registers.pc =
                                (self.registers.pc as i32 + 8 + self.convert_u24_to_i32(n) * 4)
                                    as u32;

                            // 2S + 1N
                        }

                        // BX
                        "????_0001_0010_1111_1111_1111_0001_nnnn" => {
                            if n == 15 {
                                self.registers.pc += 8;
                                // 2S + 1N
                            }

                            let register_value = self.get_register_value(n as usize);

                            if register_value % 2 == 1 {
                                self.set_cpsr_bit(T_FLAG, true);
                                self.registers.pc = register_value & !1;
                            } else {
                                self.set_cpsr_bit(T_FLAG, false);
                                self.registers.pc = register_value & !3;
                            }

                            // 2S + 1N
                        }

                        // SWI
                        "????_1111_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            todo!("Revisit after BIOS impl");
                            // 2S + 1N
                        }

                        // ALU (I = 1) (ror shift on imm)
                        "????_00_i_oooo_s_rrrr_dddd_ssss_nnnnnnnn" => {
                            todo!("op2 is shifted imm");
                        }

                        // ALU (I = 0, R = 0) (register is shifted by shifted immediate)
                        "????_00_0_oooo_s_rrrr_dddd_sssss_tt_0_nnnn" => {
                            todo!("op2 is shifted register shifted by imm")
                        }

                        // ALU (I = 0, R = 1) (register is shifted by shifted register)
                        "????_00_0_oooo_s_rrrr_dddd_ssss_0_tt_1_nnnn" => {
                            todo!("op2 is shifted register shifted by register")
                        }

                        _ => {
                            error!("Invalid error detected: {:b}", instruction);
                        }
                    }
                } else {
                    self.registers.pc += 4;
                }
            }
            CPUState::Thumb => {}
        }

        // decode

        // execute

        clk
    }
}
