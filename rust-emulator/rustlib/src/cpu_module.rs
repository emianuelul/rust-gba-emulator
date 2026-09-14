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

#[derive(Hash, Eq, PartialEq, Debug)]
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

                if index == 13 { value.0 } else { value.1 }
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

    fn set_cpsr_bit(&mut self, index: usize, value: u8) {
        if value == 1 {
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

// ARM ALU Logic
impl CPU {
    fn convert_u24_to_i32(&self, value: u32) -> i32 {
        ((value << 8) as i32) >> 8
    }

    fn get_arm_operand_value(&self, index: usize, i: u8, r: u8) -> u32 {
        if index == PC {
            if i == 0 && r == 1 {
                self.registers.pc + 8
            } else {
                self.registers.pc + 4
            }
        } else {
            self.get_register_value(index)
        }
    }

    fn alu_set_n_z_flags(&mut self, data: u32) {
        let z_bit: u8 = if data == 0 { 1 } else { 0 };
        let n_bit: u8 = ((data >> 31) & 1) as u8;

        self.set_cpsr_bit(Z_FLAG, z_bit);
        self.set_cpsr_bit(N_FLAG, n_bit);
    }

    fn rrx(&mut self, to_shift: u32) -> u32 {
        let old_c = self.get_cpsr_bit(C_FLAG);
        let lsb = (to_shift & 1) as u8;
        self.set_cpsr_bit(C_FLAG, lsb);

        (to_shift >> 1) | ((old_c as u32) << 31)
    }

    fn execute_alu_op(&mut self, opcode: u8, rn_value: u32, rd: u8, op2: u32, s: u8) {
        let mode: CPUMode = self.get_cpu_mode();

        let s: u8 = if s == 1 && rd == 15 {
            if mode != CPUMode::Sys && mode != CPUMode::User {
                self.cpsr = if let Some(&value) = self.spsr.get(&mode) {
                    value
                } else {
                    self.cpsr
                };

                0
            } else {
                error!("CPUMode is {:?} and Rd = PC", mode);

                s
            }
        } else {
            s
        };

        match opcode {
            // AND
            0x0 => {
                let data = rn_value & op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                }

                self.set_register_value(rd as usize, data);
            }

            // EOR
            0x1 => {
                let data = rn_value ^ op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                }

                self.set_register_value(rd as usize, data);
            }

            // SUB
            0x2 => {
                let data = rn_value.wrapping_sub(op2);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = !rn_value.overflowing_sub(op2).1 as u8;
                    let v_bit = i32::overflowing_sub(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd as usize, data);
            }

            // RSB
            0x3 => {
                let data = op2.wrapping_sub(rn_value);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = !op2.overflowing_sub(rn_value).1 as u8;
                    let v_bit = i32::overflowing_sub(op2 as i32, rn_value as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd as usize, data);
            }

            // ADD
            0x4 => {
                let data = rn_value.wrapping_add(op2);

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    let c_bit = rn_value.overflowing_add(op2).1 as u8;
                    let v_bit = i32::overflowing_add(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd as usize, data);
            }

            // ADC
            0x5 => {
                let data = rn_value
                    .wrapping_add(op2)
                    .wrapping_add(self.get_cpsr_bit(C_FLAG) as u32);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let (sum1, c1) = rn_value.overflowing_add(op2);
                    let (_, c2) = sum1.overflowing_add(self.get_cpsr_bit(C_FLAG) as u32);
                    let c_bit = c1 || c2;

                    let (sum1, v1) = i32::overflowing_add(rn_value as i32, op2 as i32);
                    let (_, v2) = i32::overflowing_add(sum1, self.get_cpsr_bit(C_FLAG) as i32);
                    let v_bit = v1 || v2;

                    self.set_cpsr_bit(C_FLAG, c_bit as u8);
                    self.set_cpsr_bit(V_FLAG, v_bit as u8);
                }

                self.set_register_value(rd as usize, data);
            }

            // SBC
            0x6 => {
                let data = rn_value
                    .wrapping_sub(op2)
                    .wrapping_add((self.get_cpsr_bit(C_FLAG) as u32).wrapping_sub(1));

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let (sub1, c1) = rn_value.overflowing_sub(op2);
                    let (_, c2) =
                        sub1.overflowing_add((self.get_cpsr_bit(C_FLAG) as u32).wrapping_sub(1));
                    let c_bit = !c1 || c2;

                    let (sub1, v1) = i32::overflowing_sub(rn_value as i32, op2 as i32);
                    let (_, v2) = i32::overflowing_add(
                        sub1,
                        (self.get_cpsr_bit(C_FLAG) as i32).wrapping_sub(1),
                    );
                    let v_bit = v1 || v2;

                    self.set_cpsr_bit(C_FLAG, c_bit as u8);
                    self.set_cpsr_bit(V_FLAG, v_bit as u8);
                }

                self.set_register_value(rd as usize, data);
            }

            // RSC
            0x7 => {
                let data = op2
                    .wrapping_sub(rn_value)
                    .wrapping_add((self.get_cpsr_bit(C_FLAG) as u32).wrapping_sub(1));

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let (sub1, c1) = op2.overflowing_sub(rn_value);
                    let (_, c2) =
                        sub1.overflowing_add((self.get_cpsr_bit(C_FLAG) as u32).wrapping_sub(1));
                    let c_bit = !c1 || c2;

                    let (sub1, v1) = i32::overflowing_sub(op2 as i32, rn_value as i32);
                    let (_, v2) = i32::overflowing_add(
                        sub1,
                        (self.get_cpsr_bit(C_FLAG) as i32).wrapping_sub(1),
                    );
                    let v_bit = v1 || v2;

                    self.set_cpsr_bit(C_FLAG, c_bit as u8);
                    self.set_cpsr_bit(V_FLAG, v_bit as u8);
                }

                self.set_register_value(rd as usize, data);
            }

            // TST
            0x8 => {
                let data = rn_value & op2;

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                } else if s == 1 && rd == 15 {
                    warn!("TSTP in User/Sys mode (not allowed)");
                }
            }

            // TEQ
            0x9 => {
                let data = rn_value ^ op2;

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                } else if s == 1 && rd == 15 {
                    warn!("TEQP in User/Sys mode (not allowed)")
                }
            }

            // CMP
            0xA => {
                let data = rn_value.wrapping_sub(op2);

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                    let c_bit = !rn_value.overflowing_sub(op2).1 as u8;
                    let v_bit = i32::overflowing_sub(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                } else if s == 1 && rd == 15 {
                    warn!("CMPP in User/Sys mode (not allowed)")
                }
            }

            // CMN
            0xB => {
                let data = rn_value.wrapping_add(op2);

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                    let c_bit = rn_value.overflowing_add(op2).1 as u8;
                    let v_bit = i32::overflowing_add(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                } else if s == 1 && rd == 15 {
                    warn!("CMNP in User/Sys mode (not allowed)")
                }
            }

            // ORR
            0xC => {
                let data = rn_value | op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                }

                self.set_register_value(rd as usize, data);
            }

            // MOV
            0xD => {
                if s == 1 {
                    self.alu_set_n_z_flags(op2);
                }

                self.set_register_value(rd as usize, op2);
            }

            // BIC
            0xE => {
                let data = rn_value & !op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                }

                self.set_register_value(rd as usize, data);
            }

            // MVN
            0xF => {
                if s == 1 {
                    self.alu_set_n_z_flags(!op2);
                }

                self.set_register_value(rd as usize, !op2);
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn apply_alu_shift(
        &mut self,
        shift_type: u8,
        to_shift: u32,
        amount: u8,
        imm: bool,
        s: u8,
    ) -> u32 {
        match shift_type {
            0 => {
                if amount == 0 {
                    to_shift
                } else if amount >= 32 {
                    if s == 1 {
                        if amount > 32 {
                            self.set_cpsr_bit(C_FLAG, 0);
                        } else {
                            self.set_cpsr_bit(C_FLAG, (to_shift & 1) as u8);
                        }
                    }
                    0
                } else {
                    if s == 1 {
                        let carry_bit = ((to_shift >> (32 - amount)) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    to_shift << amount
                }
            }

            1 => {
                if amount >= 32 || (amount == 0 && imm) {
                    if s == 1 {
                        let carry_bit = ((to_shift >> 31) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    0
                } else if amount == 0 {
                    to_shift
                } else {
                    if s == 1 {
                        let carry_bit = ((to_shift >> (amount - 1)) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    to_shift >> amount
                }
            }

            2 => {
                if amount >= 32 || (amount == 0 && imm) {
                    if s == 1 {
                        let carry_bit = ((to_shift >> 31) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    let first_bit = (to_shift >> 31) & 1;
                    if first_bit == 1 { u32::MAX } else { 0 }
                } else if amount == 0 {
                    to_shift
                } else {
                    if s == 1 {
                        let carry_bit = ((to_shift >> (amount - 1)) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    (to_shift as i32 >> amount) as u32
                }
            }

            3 => {
                if amount == 0 && imm {
                    self.rrx(to_shift)
                } else if amount == 0 {
                    to_shift
                } else {
                    if s == 1 {
                        let carry_bit = ((to_shift >> (amount - 1)) & 1) as u8;
                        self.set_cpsr_bit(C_FLAG, carry_bit);
                    }
                    to_shift.rotate_right(amount as u32)
                }
            }
            _ => unreachable!(),
        }
    }
}

// Step Logic
// TODO: Revisit after waitcnt
impl CPU {
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

                self.registers.pc += 4;

                #[bitmatch]
                let "cccc_????????????????????????????" = instruction;

                if self.check_condition(c as u8) {
                    #[bitmatch]
                    match instruction {
                        // B
                        "????_101_0_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            self.registers.pc =
                                (self.registers.pc as i32 + 4 + self.convert_u24_to_i32(n) * 4)
                                    as u32;

                            // 2S + 1N
                        }

                        // BL
                        "????_101_1_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            self.set_register_value(LR, self.registers.pc);
                            self.registers.pc =
                                (self.registers.pc as i32 + 4 + self.convert_u24_to_i32(n) * 4)
                                    as u32;

                            // 2S + 1N
                        }

                        // BX
                        "????_0001_0010_1111_1111_1111_0001_nnnn" => {
                            if n == 15 {
                                self.registers.pc += 4;
                                // 2S + 1N
                            }

                            let register_value = self.get_register_value(n as usize);

                            if register_value % 2 == 1 {
                                self.set_cpsr_bit(T_FLAG, 1);
                                self.registers.pc = register_value & !1;
                            } else {
                                self.set_cpsr_bit(T_FLAG, 0);
                                self.registers.pc = register_value & !3;
                            }

                            // 2S + 1N
                        }

                        // SWI
                        "????_1111_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            todo!("Revisit after BIOS impl");
                            // 2S + 1N
                        }

                        // alu (i = 1)
                        "????_00_1_oooo_s_rrrr_dddd_hhhh_nnnnnnnn" => {
                            let opcode = o as u8;
                            let rn = self.get_arm_operand_value(r as usize, 1, 0);
                            let rd = d as u8;
                            let imm = n;
                            let op2 = imm.rotate_right(h * 2);

                            if s == 1 && h != 0 {
                                let carry_bit = (op2 >> 31) as u8;
                                self.set_cpsr_bit(C_FLAG, carry_bit);
                            }

                            self.execute_alu_op(opcode, rn, rd, op2, s as u8);
                        }

                        // alu (i = 0, r = 0)
                        "????_00_0_oooo_s_rrrr_dddd_hhhhh_tt_0_nnnn" => {
                            let opcode = o as u8;
                            let rn = self.get_arm_operand_value(r as usize, 0, 0);
                            let rd = d as u8;
                            let rm = self.get_arm_operand_value(n as usize, 0, 0);
                            let shift = h;
                            let shift_type = t;
                            let op2 = self.apply_alu_shift(
                                shift_type as u8,
                                rm,
                                shift as u8,
                                true,
                                s as u8,
                            );

                            self.execute_alu_op(opcode, rn, rd, op2, s as u8);
                        }

                        // alu (i = 0, r = 1)
                        "????_00_0_oooo_s_rrrr_dddd_hhhh_0_tt_1_nnnn" => {
                            let opcode = o as u8;
                            let rn = self.get_arm_operand_value(r as usize, 0, 1);
                            let rd = d as u8;
                            let rm = self.get_arm_operand_value(n as usize, 0, 1);
                            let rs = self.get_register_value(h as usize) & 0xff;
                            let shift_type = t;
                            let op2 = self.apply_alu_shift(
                                shift_type as u8,
                                rm,
                                rs as u8,
                                false,
                                s as u8,
                            );

                            self.execute_alu_op(opcode, rn, rd, op2, s as u8);
                        }

                        _ => {
                            error!("Invalid error detected: {:b}", instruction);
                        }
                    }
                } else {
                    todo!("add clk + 1S");
                    // clk +1S
                }
            }
            CPUState::Thumb => {}
        }

        // decode

        // execute

        clk
    }
}
