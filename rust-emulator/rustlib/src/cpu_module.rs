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

// REMINDER: PC IS ADVANCED AFTER FETCH; OPERATIONS USE, BASICALLY, THE OLD PC

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

    fn set_user_register_value(&mut self, index: usize, data: u32) {
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

    fn get_user_register_value(&self, index: usize) -> u32 {
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

// ARM B, BX Logic
impl CPU {
    fn b_convert_u24_to_i32(&self, value: u32) -> i32 {
        ((value << 8) as i32) >> 8
    }

    fn b_execute_op(&mut self, op: u8, n: u32) {
        if op == 0 {
            // B
            self.registers.pc =
                (self.registers.pc as i32 + 4 + self.b_convert_u24_to_i32(n) * 4) as u32;

            // 2S + 1N
        } else {
            // BX
            self.set_register_value(LR, self.registers.pc);
            self.registers.pc =
                (self.registers.pc as i32 + 4 + self.b_convert_u24_to_i32(n) * 4) as u32;

            // 2S + 1N
        }
    }

    fn b_execute_bx(&mut self, n: u8) {
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
}

// ARM ALU Logic
impl CPU {
    fn alu_get_arm_operand_value(&self, index: usize, i: u8, r: u8) -> u32 {
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

    fn alu_rrx(&mut self, to_shift: u32) -> u32 {
        let old_c = self.get_cpsr_bit(C_FLAG);
        let lsb = (to_shift & 1) as u8;
        self.set_cpsr_bit(C_FLAG, lsb);

        (to_shift >> 1) | ((old_c as u32) << 31)
    }

    fn alu_execute_op(&mut self, opcode: u8, rn_value: u32, rd: u8, op2: u32, s: u8) {
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

    fn alu_apply_shift(
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
                    if first_bit == 1 {
                        u32::MAX
                    } else {
                        0
                    }
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
                    self.alu_rrx(to_shift)
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

// ARM MUL Logic
impl CPU {
    fn mul_execute_op(&mut self, op: u8, rd: u8, rn: u8, rs: u8, rm: u8, s: u8) -> u32 {
        let rs_value = self.get_register_value(rs as usize);
        let rm_value = self.get_register_value(rm as usize);

        // rd > RdHi
        // rn > RdLo

        match op {
            // MUL
            0b0000 => {
                if rd == rm || rd == PC as u8 || rs == PC as u8 || rm == PC as u8 {
                    error!("MUL called with invalid args (Rd is Rm or Arg is PC)");
                    return 0;
                }
                let data: u32 = rs_value.wrapping_mul(rm_value);

                self.set_register_value(rd as usize, data);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 31) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + mI
                0
            }

            // MLA
            0b0001 => {
                if rd == rm || rn == PC as u8 || rd == PC as u8 || rs == PC as u8 || rm == PC as u8
                {
                    error!("MLA called with invalid args (Rd is Rm or Arg is PC)");
                    return 0;
                }

                let rn_value = self.get_register_value(rn as usize);
                let data: u32 = rs_value.wrapping_mul(rm_value).wrapping_add(rn_value);

                self.set_register_value(rd as usize, data);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 31) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + (m + 1)I
                0
            }

            // UMULL
            0b0100 => {
                if rd == PC as u8
                    || rn == PC as u8
                    || rm == PC as u8
                    || rd == rn
                    || rd == rm
                    || rs == rm
                {
                    error!(
                        "UMULL called with invalid args (Rd Rn and Rm may be the same || may be PC)"
                    );
                    return 0;
                }

                let data: u64 = rm_value as u64 * rs_value as u64;
                let hi: u32 = (data >> 32) as u32;
                let lo: u32 = ((data << 32) >> 32) as u32;

                self.set_register_value(rd as usize, hi);
                self.set_register_value(rn as usize, lo);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 63) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + (m + 1)I
                0
            }

            // UMLAL
            0b0101 => {
                if rd == PC as u8
                    || rn == PC as u8
                    || rm == PC as u8
                    || rd == rn
                    || rd == rm
                    || rs == rm
                {
                    error!(
                        "UMLAL called with invalid args (Rd Rn and Rm may be the same || may be PC)"
                    );
                    return 0;
                }

                let stored_hilo: u64 = (self.get_register_value(rd as usize) as u64) << 32
                    | (self.get_register_value(rn as usize) as u64);

                let data: u64 = (rm_value as u64)
                    .wrapping_mul(rs_value as u64)
                    .wrapping_add(stored_hilo);
                let hi: u32 = (data >> 32) as u32;
                let lo: u32 = ((data << 32) >> 32) as u32;

                self.set_register_value(rd as usize, hi);
                self.set_register_value(rn as usize, lo);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 63) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + (m+2)I
                0
            }

            // SMULL
            0b0110 => {
                if rd == PC as u8
                    || rn == PC as u8
                    || rm == PC as u8
                    || rd == rn
                    || rd == rm
                    || rs == rm
                {
                    error!(
                        "SMULL called with invalid args (Rd Rn and Rm may be the same || may be PC)"
                    );
                    return 0;
                }

                let data: u64 = (rm_value as i32 as u64).wrapping_mul(rs_value as i32 as u64);
                let hi: u32 = (data >> 32) as u32;
                let lo: u32 = ((data << 32) >> 32) as u32;

                self.set_register_value(rd as usize, hi);
                self.set_register_value(rn as usize, lo);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 63) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + (m+1)I
                0
            }

            // SMLAL
            0b0111 => {
                if rd == PC as u8
                    || rn == PC as u8
                    || rm == PC as u8
                    || rd == rn
                    || rd == rm
                    || rs == rm
                {
                    error!(
                        "SMLAL called with invalid args (Rd Rn and Rm may be the same || may be PC)"
                    );

                    return 0;
                }

                let stored_hilo: i64 = ((self.get_register_value(rd as usize) as u64) << 32
                    | (self.get_register_value(rn as usize) as u64))
                    as i64;

                let data: u64 = (rm_value as i32 as u64 as i64)
                    .wrapping_mul(rs_value as i32 as u64 as i64)
                    .wrapping_add(stored_hilo) as u64;
                let hi: u32 = (data >> 32) as u32;
                let lo: u32 = ((data << 32) >> 32) as u32;

                self.set_register_value(rd as usize, hi);
                self.set_register_value(rn as usize, lo);

                if s == 1 {
                    let z_bit: u8 = if data == 0 { 1 } else { 0 };
                    let n_bit: u8 = ((data >> 63) & 1) as u8;

                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(N_FLAG, n_bit);
                }

                // 1S + (m+2)I
                0
            }
            _ => {
                error!("Unsupported MUL opcode: {:b}", op);
                0
            }
        }
    }
}

// ARM PSR Transfer Logic
impl CPU {
    fn psrt_get_new_psr(
        &self,
        psr: u32,
        op: u32,
        flags: u8,
        _status: u8,
        _extension: u8,
        control: u8,
    ) -> u32 {
        let op_bytes = op.to_be_bytes();
        let psr_bytes = psr.to_be_bytes();
        let curr_mode = self.get_cpu_mode();

        let byte_mask: u8 = 0b00100000;

        let first_byte = if flags == 1 {
            psr_bytes[0] as u32
        } else {
            op_bytes[0] as u32
        };
        let second_byte = psr_bytes[1] as u32;
        let third_byte = psr_bytes[2] as u32;
        let fourth_byte = if control == 1 && curr_mode != CPUMode::User {
            psr_bytes[3] as u32
        } else {
            ((op_bytes[3] & !byte_mask) | (psr_bytes[3] & byte_mask)) as u32
        };

        first_byte << 24 | second_byte << 16 | third_byte << 8 | fourth_byte
    }

    fn psrt_execute_msr_op(&mut self, p: u8, op: u32, write_arr: [u8; 4]) {
        let curr_mode = self.get_effective_cpu_mode();
        if p == 1 && curr_mode == CPUMode::UserSys {
            error!("Called MSR with SPSR but current mode is User/Sys");
            return;
        }

        let psr = if p == 0 {
            self.cpsr
        } else {
            *self.spsr.get(&self.get_effective_cpu_mode()).unwrap()
        };

        let new_psr: u32 = self.psrt_get_new_psr(
            psr,
            op,
            write_arr[0],
            write_arr[1],
            write_arr[2],
            write_arr[3],
        );

        if p == 0 {
            self.cpsr = new_psr;
        } else {
            self.spsr.insert(self.get_effective_cpu_mode(), new_psr);
        }
    }

    fn psrt_execute_mrs_op(&mut self, p: u8, rd: u8) {
        let curr_mode = self.get_effective_cpu_mode();
        if p == 1 && curr_mode == CPUMode::UserSys {
            error!("Called MRS with SPSR in User/Sys mode");
            return;
        }

        let psr = if p == 0 {
            self.cpsr
        } else {
            *self.spsr.get(&self.get_effective_cpu_mode()).unwrap()
        };

        self.set_register_value(rd as usize, psr);
    }
}

// ARM Single Data Transfer
impl CPU {
    fn sdt_rrx(&mut self, to_shift: u32) -> u32 {
        let old_c = self.get_cpsr_bit(C_FLAG);

        (to_shift >> 1) | ((old_c as u32) << 31)
    }

    fn sdt_apply_shift(&mut self, to_shift: u32, amount: u8, shift_type: u8) -> u32 {
        match shift_type {
            0 => {
                if amount == 0 {
                    to_shift
                } else if amount >= 32 {
                    0
                } else {
                    to_shift << amount
                }
            }

            1 => {
                if amount >= 32 || amount == 0 {
                    0
                } else if amount == 0 {
                    to_shift
                } else {
                    to_shift >> amount
                }
            }

            2 => {
                if amount >= 32 || amount == 0 {
                    let first_bit = (to_shift >> 31) & 1;
                    if first_bit == 1 {
                        u32::MAX
                    } else {
                        0
                    }
                } else if amount == 0 {
                    to_shift
                } else {
                    (to_shift as i32 >> amount) as u32
                }
            }

            3 => {
                if amount == 0 {
                    self.sdt_rrx(to_shift)
                } else {
                    to_shift.rotate_right(amount as u32)
                }
            }
            _ => unreachable!(),
        }
    }

    fn sdt_get_aligned_addr(&self, addr: u32) -> u32 {
        addr & !0b11
    }

    // read.1 and write return clk times from mem acc

    fn sdt_read_data(&self, memory: &mut GBAMemory, addr: u32, byte_word: u8) -> (u32, u32) {
        if !addr.is_multiple_of(4) {
            if byte_word == 0 {
                let result = memory.read32(self.sdt_get_aligned_addr(addr));
                (result.0.rotate_right(8 * (addr % 4)), result.1)
            } else {
                let result = memory.read8(addr);
                (result.0 as u32, result.1)
            }
        } else {
            if byte_word == 0 {
                memory.read32(addr)
            } else {
                let result = memory.read8(addr);
                (result.0 as u32, result.1)
            }
        }
    }

    fn sdt_write_data(&self, memory: &mut GBAMemory, addr: u32, data: u32, byte_word: u8) -> u32 {
        if !addr.is_multiple_of(4) {
            if byte_word == 0 {
                let addr = self.sdt_get_aligned_addr(addr);
                memory.write32(addr, data)
            } else {
                memory.write8(addr, data as u8)
            }
        } else {
            if byte_word == 0 {
                memory.write32(addr, data)
            } else {
                memory.write8(addr, data as u8)
            }
        }
    }

    // flags
    // 0 - P (pre / post indexing)
    // 1 - U (up / down bit)
    // 2 - B (byte / word)
    // 3 - X (writeback)
    fn sdt_execute_ldr(
        &mut self,
        memory: &mut GBAMemory,
        flags: [u8; 4],
        rn: usize,
        rd: usize,
        operand: u32,
    ) -> u32 {
        let mut clk: u32 = 0;
        let rn_value = if rn == PC {
            self.registers.pc + 4
        } else {
            self.get_register_value(rn)
        };

        let read_addr;
        if flags[0] == 0 {
            // post indexing
            read_addr = rn_value;

            let (data, io_clk) = self.sdt_read_data(memory, read_addr, flags[2]);

            self.set_register_value(rd, data);

            self.set_register_value(rn, (read_addr as i32 + operand as i32) as u32);

            clk += io_clk;
        } else {
            // pre indexing
            read_addr = (rn_value as i32 + operand as i32) as u32;

            let (data, io_clk) = self.sdt_read_data(memory, read_addr, flags[2]);

            self.set_register_value(rd, data);

            if flags[3] == 1 {
                self.set_register_value(rn, read_addr);
            }

            clk += io_clk;
        }

        // clk + 1S + 1N + 1I
        clk
    }

    fn sdt_execute_str(
        &mut self,
        memory: &mut GBAMemory,
        flags: [u8; 4],
        rn: usize,
        rd: usize,
        operand: u32,
    ) -> u32 {
        let mut clk: u32 = 0;

        let rn_value = if rn == PC {
            self.registers.pc + 4
        } else {
            self.get_register_value(rn)
        };

        let rd_value = if rd == PC {
            self.registers.pc + 8
        } else {
            self.get_register_value(rd)
        };

        let data = rd_value;

        if flags[0] == 0 {
            // post indexing
            let addr = rn_value;

            let io_clk = self.sdt_write_data(memory, addr, data, flags[2]);

            self.set_register_value(rn, (addr as i32 + operand as i32) as u32);

            clk += io_clk;
        } else {
            // pre indexing
            let addr = (rn_value as i32 + operand as i32) as u32;

            let io_clk = self.sdt_write_data(memory, addr, data, flags[2]);

            if flags[3] == 1 {
                self.set_register_value(rn, addr);
            }

            clk += io_clk;
        }

        // clk + 2N
        clk
    }

    fn sdt_execute_op(
        &mut self,
        memory: &mut GBAMemory,
        rn: usize,
        rd: usize,
        opcode: u8,
        flags: [u8; 4],
        operand: u32,
    ) -> u32 {
        if opcode == 0 {
            self.sdt_execute_str(memory, flags, rn, rd, operand)
        } else {
            self.sdt_execute_ldr(memory, flags, rn, rd, operand)
        }
    }
}

// ARM HSDT
impl CPU {
    fn hsdt_strh(
        &mut self,
        memory: &mut GBAMemory,
        rn: usize,
        rd: usize,
        offset: i32,
        flags: [u8; 4],
    ) -> u32 {
        let mut clk: u32 = 0;

        let rn_value = if rn == PC {
            self.registers.pc + 4
        } else {
            self.get_register_value(rn)
        };
        let rd_value = if rd == PC {
            self.registers.pc + 8
        } else {
            self.get_register_value(rd)
        };

        let addr = if flags[0] == 0 {
            rn_value
        } else {
            (rn_value as i32 + offset) as u32
        };

        clk += memory.write16(addr, rd_value as u16);

        if flags[0] == 0 {
            self.set_register_value(rn, (addr as i32 + offset) as u32);
        } else {
            if flags[2] == 1 {
                self.set_register_value(rn, addr);
            }
        }

        // clk + 2N
        clk
    }

    fn hsdt_ldr_op(
        &mut self,
        memory: &mut GBAMemory,
        rn: usize,
        rd: usize,
        offset: i32,
        opcode: u8,
        flags: [u8; 4],
    ) -> u32 {
        let mut clk: u32 = 0;

        let rn_value = if rn == PC {
            // clk += (1S + 1N)
            self.registers.pc + 4
        } else {
            self.get_register_value(rn)
        };

        let addr = if flags[0] == 0 {
            rn_value
        } else {
            (rn_value as i32 + offset) as u32
        };

        let (data, io) = if opcode == 0b01 {
            let result = memory.read16(addr);
            (result.0 as u32, result.1)
        } else if opcode == 0b10 {
            let result = memory.read8(addr);
            (result.0 as i8 as i32 as u32, result.1)
        } else {
            let result = memory.read16(addr);
            (result.0 as i16 as i32 as u32, result.1)
        };

        clk += io;

        self.set_register_value(rd, data);

        if flags[0] == 0 {
            self.set_register_value(rn, (addr as i32 + offset) as u32);
        } else {
            if flags[2] == 1 {
                self.set_register_value(rn, addr);
            }
        }

        // clk += (1S + 1N + 1I)
        clk
    }

    // p - pre-post
    // u - up-down
    // w - writeback
    // l - load-store
    fn hsdt_execute_op(
        &mut self,
        memory: &mut GBAMemory,
        flags: [u8; 4],
        rn: usize,
        rd: usize,
        opcode: u8,
        offset: i32,
    ) -> u32 {
        let mut clk = 0;

        if flags[3] == 0 {
            match opcode {
                // STRH
                0b01 => {
                    clk += self.hsdt_strh(memory, rn, rd, offset, flags);
                }

                _ => {
                    error!("Opcode: {:b} used in store mode", opcode);
                }
            }
        } else {
            match opcode {
                0b00 => {
                    warn!("Reserved opcode for L = 1: {:b}", opcode);
                }

                // LDRH | LDRSB | LDRSH
                0b01..=0b11 => {
                    clk += self.hsdt_ldr_op(memory, rn, rd, offset, opcode, flags);
                }

                _ => {
                    unreachable!()
                }
            }
        }

        clk
    }
}

// ARM Block Data Transfer
impl CPU {
    fn bdt_get_start_addr(
        &self,
        block_size: usize,
        pre_post: u8,
        up_down: u8,
        rn_value: u32,
    ) -> u32 {
        match (pre_post, up_down) {
            (0, 1) => rn_value,
            (1, 1) => rn_value + 4,
            (0, 0) => rn_value - block_size as u32 + 4,
            (1, 0) => rn_value - block_size as u32,
            _ => {
                unreachable!();
            }
        }
    }

    // p - pre-post
    // u - up_down
    // s - load psr
    // w - write-back
    fn bdt_execute_op(
        &mut self,
        memory: &mut GBAMemory,
        rlist: &mut Vec<usize>,
        opcode: u8,
        flags: [u8; 4],
        rn: usize,
    ) {
        let rn_value = self.get_register_value(rn);
        let s_bit = flags[2] == 1 && self.get_effective_cpu_mode() != CPUMode::UserSys;
        let was_empty = rlist.is_empty();
        if was_empty {
            rlist.push(PC);
        }

        let block_size = 4 * rlist.len();
        let start_addr = self.bdt_get_start_addr(block_size, flags[0], flags[1], rn_value);
        let writeback_addr = if was_empty {
            if flags[1] == 1 {
                rn_value.wrapping_add(0x40)
            } else {
                rn_value.wrapping_sub(0x40)
            }
        } else {
            start_addr + block_size as u32
        };

        match opcode {
            // STM - [rn+offset] = rlist[current_index]
            0 => {
                let new_base = start_addr + block_size as u32;

                for (index, &val) in rlist.iter().enumerate() {
                    let addr: u32 = start_addr + 4 * index as u32;

                    let data: u32 = if s_bit {
                        self.get_user_register_value(val)
                    } else if val == rn && index > 0 && flags[3] == 1 {
                        new_base
                    } else {
                        self.get_register_value(val)
                    };

                    memory.write32(addr, data);
                }

                if flags[3] == 1 && !s_bit {
                    self.set_register_value(rn, writeback_addr);
                }
            }

            // LDM
            1 => {
                let change_psr = rlist.contains(&PC) && s_bit;

                for (index, &value) in rlist.iter().enumerate() {
                    let addr: u32 = start_addr + 4 * index as u32;

                    if s_bit && value == PC {
                        self.cpsr = *self.spsr.get(&self.get_effective_cpu_mode()).unwrap();
                    }

                    let data = memory.read32(addr).0;
                    if s_bit && !change_psr {
                        self.set_user_register_value(value, data);
                    } else {
                        self.set_register_value(value, data);
                    }
                }

                if flags[3] == 1 && !rlist.contains(&rn) {
                    if s_bit && !change_psr {
                        self.set_user_register_value(rn, writeback_addr);
                    } else {
                        self.set_register_value(rn, writeback_addr);
                    }
                }
            }

            _ => {
                unreachable!();
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
                        // B / BL
                        "????_101_o_nnnnnnnnnnnnnnnnnnnnnnnn" => {
                            self.b_execute_op(o as u8, n);

                            // 2S + 1N
                        }

                        // BX
                        "????_0001_0010_1111_1111_1111_0001_nnnn" => {
                            self.b_execute_bx(n as u8);

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
                            let rn = self.alu_get_arm_operand_value(r as usize, 1, 0);
                            let rd = d as u8;
                            let imm = n;
                            let op2 = imm.rotate_right(h * 2);

                            if s == 1 && h != 0 {
                                let carry_bit = (op2 >> 31) as u8;
                                self.set_cpsr_bit(C_FLAG, carry_bit);
                            }

                            self.alu_execute_op(opcode, rn, rd, op2, s as u8);

                            // (1+p)S+rI+pN
                        }

                        // alu (i = 0, r = 0)
                        "????_00_0_oooo_s_rrrr_dddd_hhhhh_tt_0_nnnn" => {
                            let opcode = o as u8;
                            let rn = self.alu_get_arm_operand_value(r as usize, 0, 0);
                            let rd = d as u8;
                            let rm = self.alu_get_arm_operand_value(n as usize, 0, 0);
                            let shift = h;
                            let shift_type = t;
                            let op2 = self.alu_apply_shift(
                                shift_type as u8,
                                rm,
                                shift as u8,
                                true,
                                s as u8,
                            );

                            self.alu_execute_op(opcode, rn, rd, op2, s as u8);

                            // (1+p)S+rI+pN
                        }

                        // alu (i = 0, r = 1)
                        "????_00_0_oooo_s_rrrr_dddd_hhhh_0_tt_1_nnnn" => {
                            let opcode = o as u8;
                            let rn = self.alu_get_arm_operand_value(r as usize, 0, 1);
                            let rd = d as u8;
                            let rm = self.alu_get_arm_operand_value(n as usize, 0, 1);
                            let rs = self.get_register_value(h as usize) & 0xff;
                            let shift_type = t;
                            let op2 = self.alu_apply_shift(
                                shift_type as u8,
                                rm,
                                rs as u8,
                                false,
                                s as u8,
                            );

                            self.alu_execute_op(opcode, rn, rd, op2, s as u8);

                            // (1+p)S+rI+pN
                        }

                        // Multiply & Multiply-Accumulate
                        "????_000_oooo_s_dddd_nnnn_ffff_1001_mmmm" => {
                            let op = o as u8;
                            let rd = d as u8;
                            let rn = n as u8;
                            let rs = f as u8;
                            let rm = m as u8;

                            self.mul_execute_op(op, rd, rn, rs, rm, s as u8);

                            // MUL - 1S + mI
                            // MLA + MULL (UMULL, SMULL) - 1S + (m+1)I
                            // MLAL (UMLAL, SMLAL) - 1S + (m+2)I
                        }

                        // PSR Transfer (i = 0, MRS)
                        "????_00_0_10_p_0_0_1111_dddd_000000000000" => {
                            self.psrt_execute_mrs_op(p as u8, d as u8);

                            // 1S
                        }

                        // PSR Transfer (i = 0, MSR)
                        "????_00_0_10_p_1_0_f_s_x_c_1111_00000000_mmmm" => {
                            let op = self.get_register_value(m as usize);

                            self.psrt_execute_msr_op(
                                p as u8,
                                op,
                                [f as u8, s as u8, x as u8, c as u8],
                            );

                            // 1S
                        }

                        // PSR Transfer (i = 1, MSR)
                        "????_00_1_10_p_1_0_f_s_x_c_1111_hhhh_iiiiiiii" => {
                            let op = i.rotate_right(h * 2);

                            self.psrt_execute_msr_op(
                                p as u8,
                                op,
                                [f as u8, s as u8, x as u8, c as u8],
                            );

                            // 1S
                        }

                        // SDT - LDR, STR (i = 0) (immediate offset)
                        "????_01_0_p_u_b_x_o_nnnn_dddd_iiiiiiiiiiii" => {
                            let imm = if u == 0 {
                                -(i as i32)
                            } else {
                                i as i32
                            };

                            self.sdt_execute_op(
                                memory,
                                n as usize,
                                d as usize,
                                o as u8,
                                [p as u8, u as u8, b as u8, x as u8],
                                imm as u32,
                            );
                        }

                        // SDT - LDR STR (i = 1) (shifted immediate offset)
                        "????_01_1_p_u_b_x_o_nnnn_dddd_iiiii_ss_0_mmmm" => {
                            if m as usize == PC {
                                error!("Called SDR/LDR with I = 1 and Rm = PC");
                            } else {
                                let rm_value = self.get_register_value(m as usize);
                                let shifted = self.sdt_apply_shift(rm_value, i as u8, s as u8);
                                let operand = if u == 0 {
                                    -(shifted as i32)
                                } else {
                                    shifted as i32
                                } as u32;

                                self.sdt_execute_op(
                                    memory,
                                    n as usize,
                                    d as usize,
                                    o as u8,
                                    [p as u8, u as u8, b as u8, x as u8],
                                    operand,
                                );
                            };
                        }

                        // HWord Signed Data Transfer (LDRH, LDRSH, LDRSB, STRH)
                        "????_000_p_u_i_w_l_nnnn_dddd_aaaa_1_oo_1_bbbb" => {
                            let offset: i32 = if i == 0 {
                                if u == 0 {
                                    -(self.get_register_value(b as usize) as i32)
                                } else {
                                    self.get_register_value(b as usize) as i32
                                }
                            } else {
                                let full_imm = a << 4 | b;
                                if u == 0 {
                                    -(full_imm as i32)
                                } else {
                                    full_imm as i32
                                }
                            };

                            self.hsdt_execute_op(
                                memory,
                                [p as u8, u as u8, w as u8, l as u8],
                                n as usize,
                                d as usize,
                                o as u8,
                                offset,
                            );
                        }

                        // Block Data Transfer (LDM, STM)
                        "????_100_p_u_s_w_o_nnnn_rrrrrrrrrrrrrrrr" => {
                            let rlist_bitmask = r as u16;

                            let mut rlist: Vec<usize> = Vec::new();
                            for i in 0..16 {
                                let bit = (rlist_bitmask >> i) & 1;
                                if bit == 1 {
                                    rlist.push(i as usize);
                                }
                            }

                            self.bdt_execute_op(
                                memory,
                                &mut rlist,
                                o as u8,
                                [p as u8, u as u8, s as u8, w as u8],
                                n as usize,
                            );
                        }

                        _ => {
                            error!("Invalid instruction detected: {:b}", instruction);
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
