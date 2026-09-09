use crate::memory_area::GBAMemory;
use std::collections::HashMap;
use tracing::error;

// fetch
//   V
// decode
//   V
// execute

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
    ARM,
    THUMB,
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

    fn get_cpu_state(&self) -> CPUState {
        if self.get_cpsr_bit(5) == 0 {
            CPUState::ARM
        } else {
            CPUState::THUMB
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
            CPUState::ARM => {}
            CPUState::THUMB => {}
        }

        // decode

        // execute

        clk
    }
}
