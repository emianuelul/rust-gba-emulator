use crate::memory_area::GBAMemory;
use std::collections::HashMap;
use tracing::{error, warn};

// fetch
//   V
// decode
//   V
// execute

const PC: usize = 15;
const N_FLAG: usize = 31;
const Z_FLAG: usize = 30;
const C_FLAG: usize = 29;
const V_FLAG: usize = 28;

const IRQ_FLAG: usize = 7;
const FIQ_FLAG: usize = 6;
const STATE_FLAG: usize = 5;

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

enum ConditionCode {
    EQ,
    NE,
    CSHS,
    CCLO,
    MI,
    PL,
    VS,
    VC,
    HI,
    LS,
    GE,
    LT,
    GT,
    LE,
    AL,
    NV,
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

impl CPU {
    fn get_cpsr_bit(&self, index: usize) -> u8 {
        let copy = self.cpsr;

        ((copy >> index) & 1) as u8
    }

    fn set_cpsr_bit(&mut self, index: usize, value: bool) {
        if value {
            self.cpsr |= 1 << index
        } else {
            self.cpsr &= !(1 << index)
        }
    }

    fn get_cpu_state(&self) -> CPUState {
        if self.get_cpsr_bit(STATE_FLAG) == 0 {
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

                if index == 13 { value.0 } else { value.1 }
            }
            PC => self.registers.pc,

            _ => {
                error!("CPU Register read index out of bounds: {}", index);
                0
            }
        }
    }

    pub fn step(&mut self, memory: &GBAMemory) -> u32 {
        if self.registers.pc >= memory.get_rom_size() as u32 {
            error!(
                "PC tried to go over allowed memory limit {:x?}",
                self.registers.pc
            );
            return 0;
        }

        let clk: u32 = 0;

        // fetch
        match self.get_cpu_state() {
            CPUState::Arm => {}
            CPUState::Thumb => {}
        }

        // decode

        // execute

        clk
    }
}
