use crate::constants::*;
use crate::cpu_module::*;
use crate::memory_area::*;
use bitmatch::bitmatch;
use tracing::{error, warn};

// ARM Debug Execute Logic
impl CPU {
    #[bitmatch]
    pub fn arm_decode_execute(&mut self, memory: &mut GBAMemory, instruction: u32) {
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
                    tracing::warn!("Called SWI (not implemented)");
                    // 2S + 1N
                }

                // PSR Transfer (i = 1, MSR)
                "????_00_1_10_p_1_0_f_s_x_c_1111_hhhh_iiiiiiii" => {
                    let op = i.rotate_right(h * 2);

                    self.psrt_execute_msr_op(p as u8, op, [f as u8, s as u8, x as u8, c as u8]);

                    // 1S
                }

                // PSR Transfer (i = 0, MRS)
                "????_00_0_10_p_0_0_1111_dddd_000000000000" => {
                    self.psrt_execute_mrs_op(p as u8, d as u8);

                    // 1S
                }

                // PSR Transfer (i = 0, MSR)
                "????_00_0_10_p_1_0_f_s_x_c_1111_00000000_mmmm" => {
                    let op = self.get_register_value(m as usize);

                    self.psrt_execute_msr_op(p as u8, op, [f as u8, s as u8, x as u8, c as u8]);

                    // 1S
                }

                // alu (i = 1)
                "????_00_1_oooo_s_rrrr_dddd_hhhh_nnnnnnnn" => {
                    let opcode = o as u8;
                    let rn = self.alu_get_arm_operand_value(r as usize, 1, 0);
                    let rd = d as usize;
                    let imm = n;
                    let op2 = imm.rotate_right(h * 2);
                    let carry_in = self.get_cpsr_bit(C_FLAG);
                    let shift_carry = if h != 0 && s == 1 {
                        Some((op2 >> 31) as u8)
                    } else {
                        None
                    };

                    self.alu_execute_op(
                        opcode,
                        (r as usize, rn),
                        rd,
                        op2,
                        s as u8,
                        (carry_in, shift_carry),
                    );

                    // (1+p)S+rI+pN
                }

                // alu (i = 0, r = 0)
                "????_00_0_oooo_s_rrrr_dddd_hhhhh_tt_0_nnnn" => {
                    let opcode = o as u8;
                    let rn = self.alu_get_arm_operand_value(r as usize, 0, 0);
                    let rd = d as usize;
                    let rm = self.alu_get_arm_operand_value(n as usize, 0, 0);
                    let shift = h;
                    let shift_type = t;
                    let carry_in = self.get_cpsr_bit(C_FLAG);
                    let (op2, shift_carry) =
                        self.alu_apply_shift(shift_type as u8, rm, shift as u8, true, s as u8);

                    self.alu_execute_op(
                        opcode,
                        (r as usize, rn),
                        rd,
                        op2,
                        s as u8,
                        (carry_in, shift_carry),
                    );

                    // (1+p)S+rI+pN
                }

                // alu (i = 0, r = 1)
                "????_00_0_oooo_s_rrrr_dddd_hhhh_0_tt_1_nnnn" => {
                    let opcode = o as u8;
                    let rn = self.alu_get_arm_operand_value(r as usize, 0, 1);
                    let rd = d as usize;
                    let rm = self.alu_get_arm_operand_value(n as usize, 0, 1);
                    let rs = self.get_register_value(h as usize) & 0xff;
                    let shift_type = t;
                    let carry_in = self.get_cpsr_bit(C_FLAG);
                    let (op2, shift_carry) =
                        self.alu_apply_shift(shift_type as u8, rm, rs as u8, false, s as u8);

                    self.alu_execute_op(
                        opcode,
                        (r as usize, rn),
                        rd,
                        op2,
                        s as u8,
                        (carry_in, shift_carry),
                    );

                    // (1+p)S+rI+pN
                }

                // SWP
                "????_00010_b_00_nnnn_dddd_00001001_mmmm" => {
                    self.swp_execute(memory, b as u8, n as usize, d as usize, m as usize);
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
                    error!("Invalid ARM instruction detected: {:b}", instruction);
                }
            }
        } else {
            // clk +1S
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
            tracing::debug!("ARM: B #{:08X}", n);
            self.registers.pc = ((self.registers.pc as i32)
                .wrapping_add(4)
                .wrapping_add(self.b_convert_u24_to_i32(n) * 4))
                as u32;

            // 2S + 1N
        } else {
            // BL
            tracing::debug!("ARM: BL #{:08X}", n);
            self.set_register_value(LR, self.registers.pc);
            self.registers.pc = ((self.registers.pc as i32)
                .wrapping_add(4)
                .wrapping_add(self.b_convert_u24_to_i32(n) * 4))
                as u32;

            // 2S + 1N
        }
    }

    fn b_execute_bx(&mut self, n: u8) {
        tracing::debug!("ARM: BX R{}", n);
        if n == 15 {
            self.registers.pc = self.registers.pc.wrapping_add(4);
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
                self.registers.pc.wrapping_add(8)
            } else {
                self.registers.pc.wrapping_add(4)
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

    fn alu_rrx(&self, to_shift: u32, s: u8) -> (u32, Option<u8>) {
        let old_c = self.get_cpsr_bit(C_FLAG);
        let result = (to_shift >> 1) | ((old_c as u32) << 31);
        let carry = if s == 1 {
            Some((to_shift & 1) as u8)
        } else {
            None
        };

        (result, carry)
    }

    fn alu_execute_op(
        &mut self,
        opcode: u8,
        rn: (usize, u32),
        rd: usize,
        op2: u32,
        s: u8,
        carry: (u8, Option<u8>),
    ) {
        let mode: CPUMode = self.get_cpu_mode();
        let carry_in = carry.0;
        let shift_carry = carry.1;

        let rn_value = rn.1;

        let s: u8 = if s == 1 && rd == 15 {
            if mode != CPUMode::Sys && mode != CPUMode::User {
                self.cpsr = if let Some(&value) = self.spsr.get(&mode) {
                    value
                } else {
                    self.cpsr
                };

                0
            } else {
                error!("ALU CPUMode is {:?} and Rd = PC", mode);

                s
            }
        } else {
            s
        };

        match opcode {
            // AND
            0x0 => {
                tracing::debug!("ARM: AND R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value & op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, data);
            }

            // EOR
            0x1 => {
                tracing::debug!("ARM: EOR R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value ^ op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, data);
            }

            // SUB
            0x2 => {
                tracing::debug!("ARM: SUB R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value.wrapping_sub(op2);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = !rn_value.overflowing_sub(op2).1 as u8;
                    let v_bit = i32::overflowing_sub(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // RSB
            0x3 => {
                tracing::debug!("ARM: RSB R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = op2.wrapping_sub(rn_value);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = !op2.overflowing_sub(rn_value).1 as u8;
                    let v_bit = i32::overflowing_sub(op2 as i32, rn_value as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // ADD
            0x4 => {
                tracing::debug!("ARM: ADD R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value.wrapping_add(op2);

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    let c_bit = rn_value.overflowing_add(op2).1 as u8;
                    let v_bit = i32::overflowing_add(rn_value as i32, op2 as i32).1 as u8;

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // ADC
            0x5 => {
                tracing::debug!("ARM: ADC R{rd} R{} Op2:{:08X}", rn.0, op2);

                let full = rn_value as u64 + op2 as u64 + carry_in as u64;
                let data = full as u32;

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = (full >> 32) as u8;
                    let v_bit = if (rn_value >> 31 == op2 >> 31) && (data >> 31 != rn_value >> 31) {
                        1
                    } else {
                        0
                    };

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // SBC
            0x6 => {
                tracing::debug!("ARM: SBC R{rd} R{} Op2:{:08X}", rn.0, op2);

                let borrow = 1 - carry_in as u32;
                let data = rn_value.wrapping_sub(op2).wrapping_sub(borrow);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = (rn_value as u64 >= op2 as u64 + borrow as u64) as u8;
                    let v_bit = if (rn_value >> 31 != op2 >> 31) && (data >> 31 != rn_value >> 31) {
                        1
                    } else {
                        0
                    };

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // RSC
            0x7 => {
                tracing::debug!("ARM: RSC R{rd} R{} Op2:{:08X}", rn.0, op2);
                let borrow = 1 - carry_in as u32;
                let data = op2.wrapping_sub(rn_value).wrapping_sub(borrow);

                if s == 1 {
                    self.alu_set_n_z_flags(data);

                    let c_bit = (op2 as u64 >= rn_value as u64 + borrow as u64) as u8;
                    let v_bit = if (op2 >> 31 != rn_value >> 31) && (data >> 31 != op2 >> 31) {
                        1
                    } else {
                        0
                    };

                    self.set_cpsr_bit(C_FLAG, c_bit);
                    self.set_cpsr_bit(V_FLAG, v_bit);
                }

                self.set_register_value(rd, data);
            }

            // TST
            0x8 => {
                tracing::debug!("ARM: TST R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value & op2;

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                } else if s == 1 && rd == 15 {
                    warn!("TSTP in User/Sys mode (not allowed)");
                }
            }

            // TEQ
            0x9 => {
                tracing::debug!("ARM: TEQ R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value ^ op2;

                if s == 1 && rd != 15 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                } else if s == 1 && rd == 15 {
                    warn!("TEQP in User/Sys mode (not allowed)")
                }
            }

            // CMP
            0xA => {
                tracing::debug!("ARM: CMP R{} Op2:{:08X}", rn.0, op2);
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
                tracing::debug!("ARM: CMN R{} Op2:{:08X}", rn.0, op2);
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
                tracing::debug!("ARM: ORR R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value | op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, data);
            }

            // MOV
            0xD => {
                tracing::debug!("ARM: MOV R{} Op2:{:08X}", rd, op2);
                if s == 1 {
                    self.alu_set_n_z_flags(op2);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, op2);
            }

            // BIC
            0xE => {
                tracing::debug!("ARM: BIC R{rd} R{} Op2:{:08X}", rn.0, op2);
                let data = rn_value & !op2;

                if s == 1 {
                    self.alu_set_n_z_flags(data);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, data);
            }

            // MVN
            0xF => {
                tracing::debug!("ARM: MVN R{} Op2:{:08X}", rd, op2);
                if s == 1 {
                    self.alu_set_n_z_flags(!op2);
                    if let Some(c) = shift_carry {
                        self.set_cpsr_bit(C_FLAG, c);
                    }
                }

                self.set_register_value(rd, !op2);
            }
            _ => {
                unreachable!()
            }
        }
    }

    fn alu_apply_shift(
        &self,
        shift_type: u8,
        to_shift: u32,
        amount: u8,
        imm: bool,
        s: u8,
    ) -> (u32, Option<u8>) {
        let mut c_bit: Option<u8> = None;
        let shifted = match shift_type {
            0 => {
                if amount == 0 {
                    to_shift
                } else if amount >= 32 {
                    if s == 1 {
                        if amount > 32 {
                            c_bit = Some(0);
                        } else {
                            c_bit = Some((to_shift & 1) as u8);
                        }
                    }
                    0
                } else {
                    if s == 1 {
                        c_bit = Some(((to_shift >> (32 - amount)) & 1) as u8);
                    }
                    to_shift << amount
                }
            }

            1 => {
                let n = if amount == 0 && imm {
                    32
                } else {
                    amount
                };
                match n {
                    0 => to_shift,
                    1..=31 => {
                        if s == 1 {
                            c_bit = Some(((to_shift >> (n - 1)) & 1) as u8);
                        }
                        to_shift >> n
                    }
                    32 => {
                        if s == 1 {
                            c_bit = Some((to_shift >> 31) as u8);
                        }
                        0
                    }
                    _ => {
                        if s == 1 {
                            c_bit = Some(0);
                        }
                        0
                    }
                }
            }

            2 => {
                if amount >= 32 || (amount == 0 && imm) {
                    if s == 1 {
                        c_bit = Some(((to_shift >> 31) & 1) as u8);
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
                        c_bit = Some(((to_shift >> (amount - 1)) & 1) as u8);
                    }
                    (to_shift as i32 >> amount) as u32
                }
            }

            3 => {
                if amount == 0 && imm {
                    let (result, carry) = self.alu_rrx(to_shift, s);
                    c_bit = carry;
                    result
                } else if amount == 0 {
                    to_shift
                } else {
                    let res = to_shift.rotate_right((amount & 31) as u32);
                    if s == 1 {
                        c_bit = Some((res >> 31) as u8);
                    }
                    res
                }
            }
            _ => unreachable!(),
        };

        (shifted, c_bit)
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
                tracing::debug!("ARM: MUL Rd{} Rm{} Rs{}", rd, rm, rs);
                if rd == PC as u8 || rs == PC as u8 || rm == PC as u8 {
                    error!("MUL called with invalid args (Arg is PC)");
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
                tracing::debug!("ARM: MLA Rd{} Rm{} Rs{} Rn{}", rd, rm, rs, rn);
                if rn == PC as u8 || rd == PC as u8 || rs == PC as u8 || rm == PC as u8 {
                    error!("MLA called with invalid args (Arg is PC)");
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
                tracing::debug!("ARM: UMULL RdLo{} RdHi{} Rm{} Rs{}", rn, rd, rm, rs);
                if rd == PC as u8 || rn == PC as u8 || rm == PC as u8 || rs == PC as u8 {
                    error!("UMULL called with invalid args (arg may not be PC)");
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
                tracing::debug!("ARM: UMLAL RdLo{} RdHi{} Rm{} Rs{}", rn, rd, rm, rs);
                if rd == PC as u8 || rn == PC as u8 || rm == PC as u8 || rs == PC as u8 {
                    error!("UMLAL called with invalid args (arg may not be PC)");
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
                tracing::debug!("ARM: SMULL RdLo{} RdHi{} Rm{} Rs{}", rn, rd, rm, rs);
                if rd == PC as u8 || rn == PC as u8 || rm == PC as u8 || rs == PC as u8 {
                    error!("SMULL called with invalid args (arg may not be PC)");
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
                tracing::debug!("ARM: SMLAL RdLo{} RdHi{} Rm{} Rs{}", rn, rd, rm, rs);
                if rd == PC as u8 || rn == PC as u8 || rm == PC as u8 || rs == PC as u8 {
                    error!("SMLAL called with invalid args (arg may not be PC)");

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
        status: u8,
        extension: u8,
        control: u8,
    ) -> u32 {
        let mut new_psr = psr;
        let privileged = self.get_cpu_mode() != CPUMode::User;

        if flags == 1 {
            if !privileged {
                new_psr = (new_psr & !0xF000_0000) | (op & 0xF000_0000);
            } else {
                new_psr = (new_psr & !0xFF00_0000) | (op & 0xFF00_0000);
            }
        }

        if status == 1 && privileged {
            new_psr = (new_psr & !0x00FF_0000) | (op & 0x00FF_0000);
        }

        if extension == 1 && privileged {
            new_psr = (new_psr & !0x0000_FF00) | (op & 0x0000_FF00);
        }

        if control == 1 && privileged {
            new_psr = (new_psr & !0x0000_00DF) | (op & 0x0000_00DF);
        }

        new_psr
    }

    fn psrt_execute_msr_op(&mut self, p: u8, op: u32, write_arr: [u8; 4]) {
        tracing::debug!(
            "ARM: MSR PSR{} Op{:08X}",
            if p == 0 {
                format!("CPSR: {}", self.cpsr)
            } else {
                format!(
                    "SPSR: {}",
                    *self.spsr.get(&self.get_effective_cpu_mode()).unwrap()
                )
            },
            op
        );

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
        tracing::debug!(
            "ARM: MRS Rd{} PSR{}",
            rd,
            if p == 0 {
                format!("CPSR: {}", self.cpsr)
            } else {
                format!(
                    "SPSR: {}",
                    *self.spsr.get(&self.get_effective_cpu_mode()).unwrap()
                )
            },
        );
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
                let n = if amount == 0 {
                    32
                } else {
                    amount
                };
                match n {
                    0 => to_shift,
                    1..=31 => to_shift >> amount,
                    32.. => 0,
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
            self.registers.pc.wrapping_add(4)
        } else {
            self.get_register_value(rn)
        };

        let read_addr;
        if flags[0] == 0 {
            // post indexing
            read_addr = rn_value;

            tracing::debug!(
                "ARM: LDR{} Rd{} <{:08X}>",
                if flags[2] == 1 {
                    "B"
                } else {
                    ""
                },
                rd,
                read_addr
            );
            let (data, io_clk) = self.sdt_read_data(memory, read_addr, flags[2]);
            let data = if rd == PC {
                data & !3
            } else {
                data
            };

            self.set_register_value(rd, data);

            if rd != rn {
                self.set_register_value(
                    rn,
                    ((read_addr as i32).wrapping_add(operand as i32)) as u32,
                );
            }

            clk += io_clk;
        } else {
            // pre indexing
            read_addr = ((rn_value as i32).wrapping_add(operand as i32)) as u32;

            tracing::debug!(
                "ARM: LDR{} Rd{} <{:08X}>",
                if flags[2] == 1 {
                    "B"
                } else {
                    ""
                },
                rd,
                read_addr
            );
            let (data, io_clk) = self.sdt_read_data(memory, read_addr, flags[2]);
            let data = if rd == PC {
                data & !3
            } else {
                data
            };

            self.set_register_value(rd, data);

            if flags[3] == 1 && rd != rn {
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
            self.registers.pc.wrapping_add(4)
        } else {
            self.get_register_value(rn)
        };

        let rd_value = if rd == PC {
            self.registers.pc.wrapping_add(8)
        } else {
            self.get_register_value(rd)
        };

        let data = rd_value;

        if flags[0] == 0 {
            // post indexing
            let addr = rn_value;

            tracing::debug!(
                "ARM: STR{} Rd{} <{:08X}>",
                if flags[2] == 1 {
                    "B"
                } else {
                    ""
                },
                rd,
                addr
            );
            let io_clk = self.sdt_write_data(memory, addr, data, flags[2]);

            self.set_register_value(rn, ((addr as i32).wrapping_add(operand as i32)) as u32);

            clk += io_clk;
        } else {
            // pre indexing
            let addr = ((rn_value as i32).wrapping_add(operand as i32)) as u32;

            tracing::debug!(
                "ARM: STR{} Rd{} <{:08X}>",
                if flags[2] == 1 {
                    "B"
                } else {
                    ""
                },
                rd,
                addr
            );
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
            self.registers.pc.wrapping_add(4)
        } else {
            self.get_register_value(rn)
        };
        let rd_value = if rd == PC {
            self.registers.pc.wrapping_add(8)
        } else {
            self.get_register_value(rd)
        };

        let addr = if flags[0] == 0 {
            rn_value
        } else {
            ((rn_value as i32).wrapping_add(offset)) as u32
        };

        tracing::debug!("ARM: STRH Rd{}, <{:08X}>", rd, addr);
        clk += memory.write16(addr, rd_value as u16);

        if flags[0] == 0 {
            self.set_register_value(rn, ((addr as i32).wrapping_add(offset)) as u32);
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
            self.registers.pc.wrapping_add(4)
        } else {
            self.get_register_value(rn)
        };

        let addr = if flags[0] == 0 {
            rn_value
        } else {
            (rn_value as i32).wrapping_add(offset) as u32
        };

        let (data, io) = if opcode == 0b01 {
            tracing::debug!("ARM: LDRH Rd{}, <{:08X}>", rd, addr);
            let result = memory.read16(addr);
            if !addr.is_multiple_of(2) {
                ((result.0 as u32).rotate_right(8), result.1)
            } else {
                (result.0 as u32, result.1)
            }
        } else if opcode == 0b10 {
            tracing::debug!("ARM: LDRSB Rd{}, <{:08X}>", rd, addr);
            let result = memory.read8(addr);
            (result.0 as i8 as i32 as u32, result.1)
        } else {
            tracing::debug!("ARM: LDRSH Rd{}, <{:08X}>", rd, addr);
            if !addr.is_multiple_of(2) {
                let result = memory.read8(addr);
                (result.0 as i8 as i32 as u32, result.1)
            } else {
                let result = memory.read16(addr);
                (result.0 as i16 as i32 as u32, result.1)
            }
        };

        clk += io;

        self.set_register_value(rd, data);

        if rd != rn {
            if flags[0] == 0 {
                self.set_register_value(rn, ((addr as i32).wrapping_add(offset)) as u32);
            } else if flags[2] == 1 {
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
                    error!("STRH Opcode: {:b} used in store mode", opcode);
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
    fn bdt_execute_op(
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

        let block_size = if was_empty {
            0x40
        } else {
            4 * rlist.len()
        };

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
                tracing::debug!("ARM: STM Rn{} Rlist:{:?}", rn, rlist);
                let mut addr = start_addr;
                for (index, &reg) in rlist.iter().enumerate() {
                    let data = if reg == PC {
                        self.registers.pc.wrapping_add(8)
                    } else if s_bit {
                        self.get_user_register_value(reg)
                    } else if reg == rn && index != 0 && flags[3] == 1 {
                        writeback_addr
                    } else {
                        self.get_register_value(reg)
                    };

                    clk += memory.write32(addr, data);
                    addr = addr.wrapping_add(4);
                }

                if flags[3] == 1 {
                    self.set_register_value(rn, writeback_addr);
                }

                // clk += (n-1)S + 2N
            }

            // LDM
            1 => {
                tracing::debug!("ARM: LDM Rn{} Rlist:{:?}", rn, rlist);
                let change_psr = !was_empty && rlist.contains(&PC) && s_bit;
                let load_user_reg = !change_psr && s_bit;

                let mut addr = start_addr;
                for &reg in rlist.iter() {
                    let (mut data, mem_clk) = memory.read32(addr);
                    clk += mem_clk;

                    if reg == PC {
                        data &= !3;
                    }

                    if load_user_reg {
                        self.set_user_register_value(reg, data);
                    } else {
                        self.set_register_value(reg, data);
                    }

                    addr = addr.wrapping_add(4);
                }

                if change_psr {
                    let saved_psr = self
                        .spsr
                        .get(&self.get_effective_cpu_mode())
                        .expect("LDM with S = 1 needs to be called from mode with SPSR");
                    self.cpsr = *saved_psr;
                }

                if flags[3] == 1 && !rlist.contains(&rn) {
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
    fn swp_execute(
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

        tracing::debug!(
            "ARM: SWP{} Rd{}, Rm{}, <{:08X}>",
            if byte_word == 1 {
                "B"
            } else {
                ""
            },
            rd,
            rm,
            rn_value
        );
        let addr = rn_value;
        let (data, mem_clk) = self.sdt_read_data(memory, addr, byte_word);
        self.set_register_value(rd, data);
        clk += mem_clk;

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
