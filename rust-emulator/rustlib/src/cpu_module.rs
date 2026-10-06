use crate::constants::*;
use crate::memory_area::GBAMemory;
use bitmatch::bitmatch;
use std::collections::HashMap;
use tracing::{error, warn};

// REMINDER: PC IS ADVANCED AFTER FETCH; OPERATIONS USE, BASICALLY, THE OLD PC

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum CPUMode {
    UserSys,
    User,
    Sys,
    Fiq,
    Supervisor,
    Abort,
    Irq,
    Undefined,
}

pub enum CPUState {
    Arm,
    Thumb,
}

// pointers: r13, r14 (SP, LR)
pub struct CPURegisters {
    pub common: Vec<u32>, // r0-r12
    pub fiq: Vec<u32>,    // r8-r12
    pub pointers: HashMap<CPUMode, (u32, u32)>,
    pub pc: u32,
}

impl CPURegisters {
    fn new() -> Self {
        CPURegisters {
            common: [0; 13].to_vec(),
            fiq: [0; 5].to_vec(),
            pointers: HashMap::from([
                (CPUMode::UserSys, (0x03007F00, 0)),
                (CPUMode::Fiq, (0, 0)),
                (CPUMode::Supervisor, (0x03007FE0, 0)),
                (CPUMode::Abort, (0, 0)),
                (CPUMode::Irq, (0x03007FA0, 0)),
                (CPUMode::Undefined, (0, 0)),
            ]),
            pc: 0x08000000,
        }
    }
}

pub struct CPU {
    pub registers: CPURegisters,
    pub cpsr: u32,
    pub spsr: HashMap<CPUMode, u32>,
}

// init
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
    pub fn set_register_value(&mut self, index: usize, data: u32) {
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

    pub fn set_user_register_value(&mut self, index: usize, data: u32) {
        match index {
            0..8 => self.registers.common[index] = data,
            8..=12 => {
                self.registers.common[index] = data;
            }
            13..=14 => {
                let value = self.registers.pointers.get(&CPUMode::UserSys).unwrap();

                if index == 13 {
                    self.registers
                        .pointers
                        .insert(CPUMode::UserSys, (data, value.1));
                } else {
                    self.registers
                        .pointers
                        .insert(CPUMode::UserSys, (value.0, data));
                }
            }
            PC => self.registers.pc = data,

            _ => {
                error!("CPU Register read index out of bounds: {}", index);
            }
        }
    }

    pub fn get_register_value(&self, index: usize) -> u32 {
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

    pub fn get_user_register_value(&self, index: usize) -> u32 {
        match index {
            0..8 => self.registers.common[index],
            8..=12 => self.registers.common[index],
            13..=14 => {
                let value = self.registers.pointers.get(&CPUMode::UserSys).unwrap();

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
    pub fn get_cpsr_bit(&self, index: usize) -> u8 {
        ((self.cpsr >> index) & 1) as u8
    }

    pub fn set_cpsr_bit(&mut self, index: usize, value: u8) {
        if value == 1 {
            self.cpsr |= 1 << index
        } else {
            self.cpsr &= !(1 << index)
        }
    }

    pub fn get_cpu_state(&self) -> CPUState {
        if self.get_cpsr_bit(T_FLAG) == 0 {
            CPUState::Arm
        } else {
            CPUState::Thumb
        }
    }

    pub fn get_cpu_mode(&self) -> CPUMode {
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

    pub fn get_effective_cpu_mode(&self) -> CPUMode {
        if [CPUMode::User, CPUMode::Sys].contains(&self.get_cpu_mode()) {
            CPUMode::UserSys
        } else {
            self.get_cpu_mode()
        }
    }

    pub fn check_condition(&self, cond_bits: u8) -> bool {
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

// CPU Fetch, Decode, Execute
impl CPU {
    fn cpu_fetch(&mut self, memory: &mut GBAMemory) -> (u32, u32) {
        match self.get_cpu_state() {
            CPUState::Arm => {
                let (instruction, read) = memory.read32(self.get_register_value(PC));
                self.registers.pc = self.registers.pc.wrapping_add(4);
                (instruction, read)
            }
            CPUState::Thumb => {
                let (instruction, read) = memory.read16(self.get_register_value(PC));
                self.registers.pc = self.registers.pc.wrapping_add(2);
                (instruction as u32, read)
            }
        }
    }

    #[bitmatch]
    fn cpu_decode_execute(&mut self, memory: &mut GBAMemory, instruction: u32) {
        match self.get_cpu_state() {
            CPUState::Arm => {
                self.arm_decode_execute(memory, instruction);
            }
            CPUState::Thumb => {
                self.thumb_decode_execute(memory, instruction);
            }
        }
    }
}

// Step Logic
// TODO: Revisit after waitcnt
// m=1 for Bit 31-8, m=2 for Bit 31-16, m=3 for Bit 31-24, and m=4 otherwise
impl CPU {
    #[bitmatch]
    pub fn step(&mut self, memory: &mut GBAMemory) -> u32 {
        let (instruction, clk) = self.cpu_fetch(memory);
        self.cpu_decode_execute(memory, instruction);

        // stubbed clk
        clk
    }
}
