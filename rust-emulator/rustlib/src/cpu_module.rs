use crate::constants::*;
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
                (CPUMode::Fiq, (0x03007F00 - 0x60, 0)),
                (CPUMode::Supervisor, (0x03007F00 - 0x60 * 2, 0)),
                (CPUMode::Abort, (0x03007F00 - 0x60 * 3, 0)),
                (CPUMode::Irq, (0x03007F00 - 0x60 * 4, 0)),
                (CPUMode::Undefined, (0x03007F00 - 0x60 * 5, 0)),
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

// THUMB Register Op Logic
impl CPU {
    fn ro_execute_move_shifted(&mut self, opcode: u8, rd: usize, rs: usize, offset: u8) {
        let rs_value = self.get_register_value(rs);

        let data: u32 = match opcode {
            // lsl
            0b00 => {
                let value = if offset == 0 {
                    rs_value
                } else if offset >= 32 {
                    0
                } else {
                    rs_value << offset
                };

                let n_bit: u8 = ((value >> 31) & 1) as u8;
                let z_bit: u8 = (value == 0) as u8;
                let c_bit: u8 = (rs_value & 1) as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);

                value
            }

            // lsr
            0b01 => {
                let value = if offset == 0 || offset >= 32 {
                    0
                } else {
                    rs_value >> offset
                };
                let n_bit: u8 = ((value >> 31) & 1) as u8;
                let z_bit: u8 = (value == 0) as u8;
                let c_bit: u8 = ((rs_value >> 31) & 1) as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                if offset != 0 {
                    self.set_cpsr_bit(C_FLAG, c_bit);
                }

                value
            }

            // asr
            0b10 => {
                let value = if offset == 0 || offset >= 32 {
                    let sign = (rs_value >> 31) & 1;
                    if sign == 1 {
                        u32::MAX
                    } else {
                        0
                    }
                } else {
                    (rs_value as i32 >> offset) as u32
                };

                let n_bit: u8 = ((value >> 31) & 1) as u8;
                let z_bit: u8 = (value == 0) as u8;
                let c_bit: u8 = ((rs_value >> 31) & 1) as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);

                value
            }

            _ => {
                unreachable!()
            }
        };

        self.set_register_value(rd, data);

        // clk = 1S
    }

    fn ro_execute_add_sub(&mut self, opcode: u8, rd: usize, rs: usize, operand: u8) {
        match opcode {
            // ADD (operand is register value)
            0b00 => {
                let old_rs_value = self.get_register_value(rs);
                let old_operand_value = self.get_register_value(operand as usize);

                let data = old_rs_value.overflowing_add(old_operand_value);
                self.set_register_value(rd, data.0);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = data.1 as u8;
                let v_bit =
                    i32::overflowing_add(old_rs_value as i32, old_operand_value as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            // SUB (operand is register value)
            0b01 => {
                let old_rs_value = self.get_register_value(rs);
                let old_operand_value = self.get_register_value(operand as usize);

                let data = old_rs_value.overflowing_sub(old_operand_value);
                self.set_register_value(rd, data.0);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = !data.1 as u8;
                let v_bit =
                    i32::overflowing_sub(old_rs_value as i32, old_operand_value as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            // ADD (operand is imm)
            0b10 => {
                let old_rs_value = self.get_register_value(rs);

                if operand == 0 {
                    // MOV
                    let data = self.get_register_value(rs);
                    self.set_register_value(rd, data);

                    let n_bit = ((data >> 31) & 1) as u8;
                    let z_bit = (data == 0) as u8;
                    let c_bit = 0;
                    let v_bit = 0;

                    self.set_cpsr_bit(N_FLAG, n_bit);
                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                } else {
                    // ADD
                    let data = old_rs_value.overflowing_add(operand as u32);
                    self.set_register_value(rd, data.0);

                    let n_bit = ((data.0 >> 31) & 1) as u8;
                    let z_bit = (data.0 == 0) as u8;
                    let c_bit = data.1 as u8;
                    let v_bit = i32::overflowing_add(old_rs_value as i32, operand as i32).1 as u8;

                    self.set_cpsr_bit(N_FLAG, n_bit);
                    self.set_cpsr_bit(Z_FLAG, z_bit);
                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }
            }

            // SUB (operand is imm)
            0b11 => {
                let old_rs_value = self.get_register_value(rs);

                let data = old_rs_value.overflowing_sub(operand as u32);
                self.set_register_value(rd, data.0);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = !data.1 as u8;
                let v_bit = i32::overflowing_sub(old_rs_value as i32, operand as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            //
            _ => {
                unreachable!()
            }
        }

        // clk = 1S
    }

    fn ro_execute_mcas_op(&mut self, opcode: u8, rd: usize, imm: u8) {
        match opcode {
            // mov
            0b00 => {
                self.set_register_value(rd, imm as u32);

                let n_bit = (((imm as u32) >> 31) & 1) as u8;
                let z_bit = (imm == 0) as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
            }

            // cmp
            0b01 => {
                let data = self.get_register_value(rd).overflowing_sub(imm as u32);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = !data.1 as u8;
                let v_bit =
                    i32::overflowing_sub(self.get_register_value(rd) as i32, imm as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            // add
            0b10 => {
                let old_rd_value = self.get_register_value(rd);

                let data = self.get_register_value(rd).overflowing_add(imm as u32);
                self.set_register_value(rd, data.0);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = data.1 as u8;
                let v_bit = i32::overflowing_add(old_rd_value as i32, imm as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            // sub
            0b11 => {
                let old_rd_value = self.get_register_value(rd);
                let data = self.get_register_value(rd).overflowing_sub(imm as u32);
                self.set_register_value(rd, data.0);

                let n_bit = ((data.0 >> 31) & 1) as u8;
                let z_bit = (data.0 == 0) as u8;
                let c_bit = !data.1 as u8;
                let v_bit = i32::overflowing_sub(old_rd_value as i32, imm as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            _ => {
                unreachable!()
            }
        }

        // clk += 1S
    }

    fn ro_set_nz_flags(&mut self, data: u32) {
        let n_bit = ((data >> 31) & 1) as u8;
        let z_bit = (data == 0) as u8;

        self.set_cpsr_bit(N_FLAG, n_bit);
        self.set_cpsr_bit(Z_FLAG, z_bit);
    }
}

// Step Logic
// TODO: Revisit after waitcnt
// m=1 for Bit 31-8, m=2 for Bit 31-16, m=3 for Bit 31-24, and m=4 otherwise
impl CPU {
    #[bitmatch]
    pub fn step(&mut self, memory: &mut GBAMemory) -> u32 {
        let mut clk: u32 = 0;

        match self.get_cpu_state() {
            CPUState::Arm => {
                // FETCH
                let (instruction, read) = memory.read32(self.get_register_value(PC));
                clk += read;

                println!(
                    "ARM STEP: PC=0x{:x}, instruction=0x{:b}",
                    self.registers.pc, instruction
                );

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
                            println!("Called SWI but is not implemented yet");
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

                            self.alu_execute_op(
                                opcode,
                                rn,
                                rd,
                                op2,
                                s as u8,
                                self.get_cpsr_bit(C_FLAG) as u32,
                            );

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

                            self.alu_execute_op(
                                opcode,
                                rn,
                                rd,
                                op2,
                                s as u8,
                                self.get_cpsr_bit(C_FLAG) as u32,
                            );

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

                            self.alu_execute_op(
                                opcode,
                                rn,
                                rd,
                                op2,
                                s as u8,
                                self.get_cpsr_bit(C_FLAG) as u32,
                            );

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

                        // SWP
                        "????_00010_b_00_nnnn_dddd_00001001_mmmm" => {
                            self.swp_execute(memory, b as u8, n as usize, d as usize, m as usize);
                        }

                        _ => {
                            error!("Invalid ARM instruction detected: {:b}", instruction);
                        }
                    }
                } else {
                    // clk +1S
                }
            }

            CPUState::Thumb => {
                let (instruction, read) = memory.read16(self.get_register_value(PC));
                clk += read;

                println!(
                    "THUMB STEP: PC=0x{:x}, instruction=0x{:x}",
                    self.registers.pc, instruction
                );

                self.registers.pc += 2;

                #[bitmatch]
                match instruction {
                    // move shifted register
                    "000_oo_nnnnn_sss_ddd" => {
                        let opcode = o as u8;
                        let offset = n as u8;
                        let rs = s as usize;
                        let rd = d as usize;

                        self.ro_execute_move_shifted(opcode, rd, rs, offset);
                    }

                    // ADD, SUB
                    "00011_oo_nnn_sss_ddd" => {
                        let opcode = o as u8;
                        let operand = n as u8;
                        let rs = s as usize;
                        let rd = d as usize;

                        self.ro_execute_add_sub(opcode, rd, rs, operand);
                    }

                    // mov, cmp, add, sub
                    "001_oo_ddd_nnnnnnnn" => {
                        let opcode = o as u8;
                        let rd = d as usize;
                        let imm = n as u8;
                        self.ro_execute_mcas_op(opcode, rd, imm);
                    }

                    // alu
                    "010000_oooo_sss_ddd" => {
                        let opcode = o as u8;
                        let rs = s as usize;
                        let rd = d as usize;

                        let rs_value = self.get_register_value(rs);
                        let rd_value = self.get_register_value(rd);
                        // let mut clk: u32 = 1S;
                        match opcode {
                            // and
                            0x0 => {
                                let data = rd_value & rs_value;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // eor
                            0x1 => {
                                let data = rd_value ^ rs_value;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // lsl
                            0x2 => {
                                let n = rs_value & 0xFF;
                                let data = match n {
                                    0 => rd_value,
                                    1..=31 => {
                                        self.set_cpsr_bit(
                                            C_FLAG,
                                            ((rd_value >> (32 - n)) & 1) as u8,
                                        );
                                        rd_value << n
                                    }
                                    32 => {
                                        self.set_cpsr_bit(C_FLAG, (rd_value & 1) as u8);
                                        0
                                    }
                                    _ => {
                                        self.set_cpsr_bit(C_FLAG, 0);
                                        0
                                    }
                                };
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // lsr
                            0x3 => {
                                let n = rs_value & 0xFF;
                                let data = match n {
                                    0 => rd_value,
                                    1..=31 => {
                                        self.set_cpsr_bit(
                                            C_FLAG,
                                            ((rd_value >> (n - 1)) & 1) as u8,
                                        );
                                        rd_value >> n
                                    }
                                    32 => {
                                        self.set_cpsr_bit(C_FLAG, (rd_value >> 31) as u8);
                                        0
                                    }
                                    _ => {
                                        self.set_cpsr_bit(C_FLAG, 0);
                                        0
                                    }
                                };
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // asr
                            0x4 => {
                                let n = rs_value & 0xFF;
                                let data = match n {
                                    0 => rd_value,
                                    1..=31 => {
                                        self.set_cpsr_bit(
                                            C_FLAG,
                                            ((rd_value >> (n - 1)) & 1) as u8,
                                        );
                                        ((rd_value as i32) >> n) as u32
                                    }
                                    _ => {
                                        self.set_cpsr_bit(C_FLAG, (rd_value >> 31) as u8);
                                        ((rd_value as i32) >> 31) as u32
                                    }
                                };
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // adc
                            0x5 => {
                                let (sum1, c1) = rd_value.overflowing_add(rs_value);
                                let (sum2, c2) =
                                    sum1.overflowing_add(self.get_cpsr_bit(C_FLAG) as u32);
                                let c_bit = (c1 || c2) as u8;

                                let v_bit = if (rd_value >> 31 == rs_value >> 31)
                                    && (sum2 >> 31 != rd_value >> 31)
                                {
                                    1
                                } else {
                                    0
                                };

                                self.ro_set_nz_flags(sum2);
                                self.set_cpsr_bit(C_FLAG, c_bit);
                                self.set_cpsr_bit(V_FLAG, v_bit);

                                self.set_register_value(rd, sum2);
                            }

                            // sbc
                            0x6 => {
                                let borrow = 1 - self.get_cpsr_bit(C_FLAG) as u32;
                                let sub2 = rd_value.wrapping_sub(rs_value).wrapping_sub(borrow);

                                let c_bit = ((rd_value as u64)
                                    >= (rs_value as u64) + (borrow as u64))
                                    as u8;

                                let v_bit = if (rd_value >> 31 != rs_value >> 31)
                                    && (sub2 >> 31 != rd_value >> 31)
                                {
                                    1
                                } else {
                                    0
                                };

                                self.ro_set_nz_flags(sub2);
                                self.set_cpsr_bit(C_FLAG, c_bit);
                                self.set_cpsr_bit(V_FLAG, v_bit);

                                self.set_register_value(rd, sub2);
                            }

                            // ror
                            0x7 => {
                                let n = rs_value & 0xFF;
                                let data = rd_value.rotate_right(n & 31);
                                self.ro_set_nz_flags(data);
                                if n != 0 {
                                    self.set_cpsr_bit(C_FLAG, (data >> 31) as u8);
                                }
                                self.set_register_value(rd, data);
                            }

                            // tst
                            0x8 => {
                                let data = rd_value & rs_value;
                                self.ro_set_nz_flags(data);
                            }

                            // neg
                            0x9 => {
                                let data = 0u32.overflowing_sub(rs_value);

                                let c_bit = !data.1 as u8;
                                let v_bit = i32::overflowing_sub(0, rs_value as i32).1 as u8;

                                self.ro_set_nz_flags(data.0);
                                self.set_cpsr_bit(C_FLAG, c_bit);
                                self.set_cpsr_bit(V_FLAG, v_bit);

                                self.set_register_value(rd, data.0);
                            }

                            // cmp
                            0xA => {
                                let data = rd_value.overflowing_sub(rs_value);

                                let c_bit = !data.1 as u8;
                                let v_bit =
                                    i32::overflowing_sub(rd_value as i32, rs_value as i32).1 as u8;

                                self.ro_set_nz_flags(data.0);
                                self.set_cpsr_bit(C_FLAG, c_bit);
                                self.set_cpsr_bit(V_FLAG, v_bit);
                            }

                            // cmn
                            0xB => {
                                let data = rd_value.overflowing_add(rs_value);

                                self.ro_set_nz_flags(data.0);
                                let c_bit = data.1 as u8;
                                let v_bit =
                                    i32::overflowing_add(rd_value as i32, rs_value as i32).1 as u8;
                                self.set_cpsr_bit(C_FLAG, c_bit);
                                self.set_cpsr_bit(V_FLAG, v_bit);
                            }

                            // orr
                            0xC => {
                                let data = rd_value | rs_value;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // mul
                            0xD => {
                                let data = rd_value.overflowing_mul(rs_value).0;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);

                                // clk += mI
                            }

                            // bic
                            0xE => {
                                let data = rd_value & !rs_value;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            // mvn
                            0xF => {
                                let data = !rs_value;
                                self.ro_set_nz_flags(data);
                                self.set_register_value(rd, data);
                            }

                            _ => {
                                unreachable!()
                            }
                        }
                    }

                    _ => {
                        error!("Invalid THUMB instructio detected: {:b}", instruction)
                    }
                }
            }
        }

        // decode

        // execute

        clk
    }
}
