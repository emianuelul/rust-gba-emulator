use crate::constants::*;
use crate::cpu_module::*;
use crate::memory_area::*;
use tracing::{error, warn};

// ARM B, BX Logic
impl CPU {
    fn b_convert_u24_to_i32(&self, value: u32) -> i32 {
        ((value << 8) as i32) >> 8
    }

    pub fn b_execute_op(&mut self, op: u8, n: u32) {
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

    pub fn b_execute_bx(&mut self, n: u8) {
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
    pub fn alu_get_arm_operand_value(&self, index: usize, i: u8, r: u8) -> u32 {
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

    pub fn alu_execute_op(&mut self, opcode: u8, rn_value: u32, rd: u8, op2: u32, s: u8) {
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

    pub fn alu_apply_shift(
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
    pub fn mul_execute_op(&mut self, op: u8, rd: u8, rn: u8, rs: u8, rm: u8, s: u8) -> u32 {
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

    pub fn psrt_execute_msr_op(&mut self, p: u8, op: u32, write_arr: [u8; 4]) {
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

    pub fn psrt_execute_mrs_op(&mut self, p: u8, rd: u8) {
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

    pub fn sdt_apply_shift(&mut self, to_shift: u32, amount: u8, shift_type: u8) -> u32 {
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
                if amount >= 32 {
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

    pub fn sdt_execute_op(
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
    pub fn hsdt_execute_op(
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
            (1, 1) => rn_value.wrapping_add(4),
            (0, 0) => rn_value.wrapping_sub(block_size as u32).wrapping_add(4),
            (1, 0) => rn_value.wrapping_sub(block_size as u32),
            _ => {
                unreachable!();
            }
        }
    }

    // p - pre-post
    // u - up_down
    // s - load psr
    // w - write-back
    pub fn bdt_execute_op(
        &mut self,
        memory: &mut GBAMemory,
        rlist: &mut Vec<usize>,
        opcode: u8,
        flags: [u8; 4],
        rn: usize,
    ) -> u32 {
        let mut clk: u32 = 0;

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
            match (flags[0], flags[1]) {
                (0, 1) | (1, 1) => rn_value.wrapping_add(block_size as u32),
                (0, 0) | (1, 0) => rn_value.wrapping_sub(block_size as u32),
                _ => unreachable!(),
            }
        };

        match opcode {
            // STM - [rn+offset] = rlist[current_index]
            0 => {
                let lowest = rlist.iter().min().copied();
                let rn_is_lowest = lowest == Some(rn);

                for (index, &val) in rlist.iter().enumerate() {
                    let addr: u32 = start_addr + 4 * index as u32;

                    let data: u32 = if s_bit {
                        self.get_user_register_value(val)
                    } else if val == rn && rn_is_lowest {
                        writeback_addr
                    } else {
                        self.get_register_value(val)
                    };

                    clk += memory.write32(addr, data);
                }

                if flags[3] == 1 && !s_bit {
                    self.set_register_value(rn, writeback_addr);
                }

                // clk += (n-1)S + 2N
            }

            // LDM
            1 => {
                let change_psr = rlist.contains(&PC) && s_bit;

                for (index, &value) in rlist.iter().enumerate() {
                    let addr: u32 = start_addr + 4 * index as u32;

                    if s_bit && value == PC {
                        // clk += 1S + 1N
                        self.cpsr = *self.spsr.get(&self.get_effective_cpu_mode()).unwrap();
                    }

                    let (data, mem_clk) = memory.read32(addr);
                    clk += mem_clk;

                    let data = if value == PC {
                        data & !3
                    } else {
                        data
                    };

                    if s_bit && !change_psr {
                        self.set_user_register_value(value, data);
                    } else {
                        self.set_register_value(value, data);
                    }
                }

                if (change_psr || !s_bit) && !rlist.contains(&rn) && flags[3] == 1 {
                    self.set_register_value(rn, writeback_addr);
                }

                // clk += nS + 1N + 1I
            }

            _ => {
                unreachable!();
            }
        }

        clk
    }
}

// ARM SWP Logic
impl CPU {
    pub fn swp_execute(
        &mut self,
        memory: &mut GBAMemory,
        byte_word: u8,
        rn: usize,
        rd: usize,
        rm: usize,
    ) -> u32 {
        let mut clk: u32 = 0;

        let rn_value = self.get_register_value(rn);
        let rm_value = self.get_register_value(rm);

        let (data, mem_clk) = if byte_word == 0 {
            memory.read32(rn_value)
        } else {
            let x = memory.read8(rn_value);
            (x.0 as u32, x.1)
        };
        clk += mem_clk;

        self.set_register_value(rd, data);

        let mem_clk = if byte_word == 0 {
            memory.write32(rn_value, rm_value)
        } else {
            memory.write8(rn_value, rm_value as u8)
        };
        clk += mem_clk;

        // 1S + 2N + 1I
        clk
    }
}
