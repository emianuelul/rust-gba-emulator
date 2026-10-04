use crate::constants::*;
use crate::cpu_module::*;
use crate::memory_area::*;
use bitmatch::bitmatch;
use tracing::error;

// THUMB Decode, Execute
impl CPU {
    #[bitmatch]
    pub fn thumb_decode_execute(&mut self, memory: &mut GBAMemory, instruction: u32) {
        #[bitmatch]
        match instruction {
            // ADD, SUB
            "00011_oo_nnn_sss_ddd" => {
                let opcode = o as u8;
                let operand = n as u8;
                let rs = s as usize;
                let rd = d as usize;

                self.ro_execute_add_sub(opcode, rd, rs, operand);
            }

            // move shifted register
            "000_oo_nnnnn_sss_ddd" => {
                let opcode = o as u8;
                let offset = n as u8;
                let rs = s as usize;
                let rd = d as usize;

                self.ro_execute_move_shifted(opcode, rd, rs, offset);
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
                self.ro_execute_alu_op(o as u8, s as usize, d as usize);
            }

            // hi register ops
            "010001_oo_a_b_ccc_ddd" => {
                self.ro_execute_hi_reg(o as u8, a as usize, b as usize, c as usize, d as usize);
            }

            // ldr (load imm from literal pool)
            "01001_ddd_nnnnnnnn" => {
                self.ls_execute_pcr(memory, d as usize, n << 2);
            }

            // load/store with register offset
            "0101_oo_0_fff_bbb_ddd" => {
                self.ls_execute_ro(memory, o as u8, f as usize, b as usize, d as usize);
            }

            // load/store sign extended byte/halfword
            "0101_oo_1_fff_bbb_ddd" => {
                self.ls_execute_sh(memory, o as u8, f as usize, b as usize, d as usize);
            }

            // load/store with imm offset
            "011_oo_nnnnn_bbb_ddd" => {
                self.ls_execute_io(memory, o as u8, n & 0b00011111, b as usize, d as usize);
            }

            // load/store halfword
            "1000_o_nnnnn_bbb_ddd" => {
                self.ls_execute_h(
                    memory,
                    o as u8,
                    (n & 0b00011111) << 1,
                    b as usize,
                    d as usize,
                );
            }

            // load/store sp relative
            "1001_o_ddd_nnnnnnnn" => {
                self.ls_execute_spr(memory, o as u8, d as usize, ((n as u8) as u32) << 2);
            }

            // get relative addr
            "1010_o_ddd_nnnnnnnn" => {
                self.ma_execute_ra(o as u8, d as usize, n << 2);
            }

            // add offset to sp
            "10110000_o_nnnnnnn" => {
                self.ma_execute_spo(o as u8, n << 2);
            }

            // push pop registers
            "1011_o_10_b_rrrrrrrr" => {
                let rlist_bitmask = r as u8;
                let mut rlist: Vec<usize> = Vec::new();
                for i in 0..8 {
                    let bit = (rlist_bitmask >> i) & 1;
                    if bit == 1 {
                        rlist.push(i as usize);
                    }
                }

                self.mls_exec_pp(memory, o as u8, b as u8, &mut rlist);
            }

            // Multiple Load Store
            "1100_o_bbb_rrrrrrrr" => {
                let rlist_bitmask = r as u8;
                let mut rlist: Vec<usize> = Vec::new();
                for i in 0..8 {
                    let bit = (rlist_bitmask >> i) & 1;
                    if bit == 1 {
                        rlist.push(i as usize);
                    }
                }

                self.mls_exec_mls(memory, o as u8, b as usize, &mut rlist);
            }

            // THUMB SWI
            "11011111_nnnnnnnn" => {
                // TODO: IMPL AFTER BIOS FUNCTIONS
                // clk += 2S + 1N
            }

            // conditional branch
            "1101_cccc_oooooooo" => {
                let cond = self.check_condition(c as u8);
                let offset = (((o as i8) as i32) << 1) as u32;

                self.jc_execute_cb(cond, offset);
            }

            // unconditional branch
            "11100_nnnnnnnnnnn" => {
                let offset = (((((n << 5) as i16) >> 5) as i32) << 1) as u32;
                let dest = self.registers.pc.wrapping_add(2).wrapping_add(offset) & !1;

                self.registers.pc = dest;

                // clk += 2S + 1N
            }

            // Long Branch 1st half: LR = PC + 4 + (nn << 12)
            "11110_nnnnnnnnnnn" => {
                let imm = ((((n << 5) as i16 as i32) >> 5) << 12) as u32;
                let data = self.registers.pc.wrapping_add(2).wrapping_add(imm);
                self.set_register_value(LR, data);

                // clk += 1S
            }

            // Long Branch 2ns half: PC = LR + (nn << 1), and LR = PC + 2 OR 1
            "11111_nnnnnnnnnnn" => {
                let imm = n << 1;
                let pc_data = self.get_register_value(LR).wrapping_add(imm) & !1;
                let lr_data = self.registers.pc | 1;

                self.registers.pc = pc_data;
                self.set_register_value(LR, lr_data);

                // clk += 2S + 1N
            }

            _ => {
                error!("invalid thumb instructio detected: {:b}", instruction)
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
                let c_bit: u8 = if offset == 0 || offset > 32 {
                    0
                } else {
                    ((rs_value >> (32 - offset)) & 1) as u8
                };

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                if offset != 0 {
                    self.set_cpsr_bit(C_FLAG, c_bit);
                }

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
                let c_bit: u8 = if offset == 0 || offset == 32 {
                    ((rs_value >> 31) & 1) as u8
                } else if offset > 32 {
                    0
                } else {
                    ((rs_value >> (offset - 1)) & 1) as u8
                };

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);

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
                let c_bit: u8 = if offset == 0 || offset == 32 {
                    ((rs_value >> 31) & 1) as u8
                } else {
                    ((rs_value >> (offset - 1)) & 1) as u8
                };

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

    fn ro_execute_alu_op(&mut self, opcode: u8, rs: usize, rd: usize) {
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
                        self.set_cpsr_bit(C_FLAG, ((rd_value >> (32 - n)) & 1) as u8);
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
                        self.set_cpsr_bit(C_FLAG, ((rd_value >> (n - 1)) & 1) as u8);
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
                        self.set_cpsr_bit(C_FLAG, ((rd_value >> (n - 1)) & 1) as u8);
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
                let (sum2, c2) = sum1.overflowing_add(self.get_cpsr_bit(C_FLAG) as u32);
                let c_bit = (c1 || c2) as u8;

                let v_bit = if (rd_value >> 31 == rs_value >> 31) && (sum2 >> 31 != rd_value >> 31)
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

                let c_bit = ((rd_value as u64) >= (rs_value as u64) + (borrow as u64)) as u8;

                let v_bit = if (rd_value >> 31 != rs_value >> 31) && (sub2 >> 31 != rd_value >> 31)
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
                let v_bit = i32::overflowing_sub(rd_value as i32, rs_value as i32).1 as u8;

                self.ro_set_nz_flags(data.0);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);
            }

            // cmn
            0xB => {
                let data = rd_value.overflowing_add(rs_value);

                self.ro_set_nz_flags(data.0);
                let c_bit = data.1 as u8;
                let v_bit = i32::overflowing_add(rd_value as i32, rs_value as i32).1 as u8;
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

    fn ro_execute_hi_reg(&mut self, opcode: u8, msbd: usize, msbs: usize, rs: usize, rd: usize) {
        let rd = msbd << 3 | rd;
        let rd_value = if rd == PC {
            self.registers.pc.wrapping_add(2)
        } else {
            self.get_register_value(rd)
        };

        let rs = msbs << 3 | rs;
        let rs_value = if rs == PC {
            self.registers.pc.wrapping_add(2)
        } else {
            self.get_register_value(rs)
        };

        match opcode {
            // add
            0b00 => {
                let data = rd_value.wrapping_add(rs_value);
                if rd == PC {
                    self.registers.pc = data & !1;
                    // clk += 2S + 1N
                } else {
                    self.set_register_value(rd, data);
                    // clk += 1S
                }
            }

            // cmp
            0b01 => {
                let tuple = rd_value.overflowing_sub(rs_value);
                let data = tuple.0;

                let n_bit = ((data >> 31) & 1) as u8;
                let z_bit = (data == 0) as u8;
                let c_bit = !tuple.1 as u8;
                let v_bit = i32::overflowing_sub(rd_value as i32, rs_value as i32).1 as u8;

                self.set_cpsr_bit(N_FLAG, n_bit);
                self.set_cpsr_bit(Z_FLAG, z_bit);
                self.set_cpsr_bit(C_FLAG, c_bit);
                self.set_cpsr_bit(V_FLAG, v_bit);

                // clk += 1S
            }

            // mov
            0b10 => {
                if rd == PC {
                    self.registers.pc = rs_value & !1;
                    // clk += 2S + 1N
                } else {
                    self.set_register_value(rd, rs_value);
                    // clk += 1S
                }
            }

            // bx
            0b11 => {
                let target = rs_value;
                if target & 1 == 0 {
                    // switch to ARM
                    self.set_cpsr_bit(T_FLAG, 0);
                    self.registers.pc = target & !3;
                } else {
                    // switch to THUMB
                    self.set_cpsr_bit(T_FLAG, 1);
                    self.registers.pc = target & !1;
                }

                // clk += 2S + 1N
            }

            _ => {
                unreachable!();
            }
        }
    }
}

// THUMB load/store logic
impl CPU {
    fn ls_execute_pcr(&mut self, memory: &mut GBAMemory, rd: usize, imm: u32) -> u32 {
        let pc_value = (self.registers.pc.wrapping_add(2)) & !2;

        let addr = pc_value.wrapping_add(imm);
        let (data, clk) = memory.read32(addr);

        self.set_register_value(rd, data);

        // clk += 1S + 1N + 1I
        clk
    }

    fn ls_execute_ro(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        offset: usize,
        rb: usize,
        rd: usize,
    ) -> u32 {
        let offset = self.get_register_value(offset);

        let addr = self.get_register_value(rb).wrapping_add(offset);
        match opcode {
            // str
            0b00 => {
                let data = self.get_register_value(rd);

                let clk: u32 = memory.write32(addr, data);

                // clk += 2N
                clk
            }

            // strb
            0b01 => {
                let data = self.get_register_value(rd) as u8;

                let clk: u32 = memory.write8(addr, data);

                // clk += 2N
                clk
            }

            // ldr
            0b10 => {
                let (data, clk) = memory.read32(addr);
                let data = data.rotate_right(8 * (addr & 3));
                self.set_register_value(rd, data);

                // clk += 1S + 1N + 1I
                clk
            }

            // ldrb
            0b11 => {
                let (data, clk) = memory.read8(addr);

                self.set_register_value(rd, data as u32);

                // clk += 1S + 1N + 1I
                clk
            }

            _ => {
                unreachable!();
            }
        }
    }

    fn ls_execute_sh(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        ro: usize,
        rb: usize,
        rd: usize,
    ) -> u32 {
        let ro_value = self.get_register_value(ro);
        let rb_value = self.get_register_value(rb);

        let addr = rb_value.wrapping_add(ro_value);
        match opcode {
            //STRH
            0b00 => {
                let data = self.get_register_value(rd) as u16;

                let clk: u32 = memory.write16(addr, data);
                // clk += 2N
                clk
            }

            // LDSB
            0b01 => {
                let (read_data, clk) = memory.read8(addr);
                let data = read_data as i8 as i32 as u32;

                self.set_register_value(rd, data);
                // clk += 1S + 1N + 1I
                clk
            }

            // LDRH
            0b10 => {
                let (data, clk) = memory.read16(addr);
                let data = if addr % 2 == 1 {
                    (data as u32).rotate_right(8)
                } else {
                    data as u32
                };

                self.set_register_value(rd, data);
                // clk += 1S + 1N + 1I
                clk
            }

            // LDSH
            0b11 => {
                if addr % 2 == 1 {
                    let (read_data, clk) = memory.read8(addr);
                    let data = read_data as i8 as i32 as u32;

                    self.set_register_value(rd, data);
                    // clk += 1S + 1N + 1I
                    clk
                } else {
                    let (read_data, clk) = memory.read16(addr);
                    let data = read_data as i16 as i32 as u32;

                    self.set_register_value(rd, data);
                    // clk += 1S + 1N + 1I
                    clk
                }
            }

            _ => {
                unreachable!();
            }
        }
    }

    fn ls_execute_io(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        imm: u32,
        rb: usize,
        rd: usize,
    ) -> u32 {
        let rb_value = self.get_register_value(rb);
        let word_addr = rb_value.wrapping_add(imm << 2);
        let byte_addr = rb_value.wrapping_add(imm);

        match opcode {
            // STR
            0b00 => {
                let data = self.get_register_value(rd);

                let clk: u32 = memory.write32(word_addr, data);

                // clk += 2N
                clk
            }

            // LDR
            0b01 => {
                let (data, clk) = memory.read32(word_addr);
                let data = data.rotate_right(8 * (word_addr & 3));

                self.set_register_value(rd, data);

                // clk += 1N+1S+1I
                clk
            }

            // STRB
            0b10 => {
                let data = self.get_register_value(rd);

                let clk: u32 = memory.write8(byte_addr, data as u8);

                // clk += 2N
                clk
            }

            // LDRB
            0b11 => {
                let (data, clk) = memory.read8(byte_addr);

                self.set_register_value(rd, data as u32);

                // clk += 1N+1S+1I
                clk
            }

            _ => {
                unreachable!();
            }
        }
    }

    fn ls_execute_h(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        imm: u32,
        rb: usize,
        rd: usize,
    ) -> u32 {
        let addr = self.get_register_value(rb).wrapping_add(imm);
        match opcode {
            // strh
            0 => {
                let data = self.get_register_value(rd) as u16;

                let clk: u32 = memory.write16(addr, data);

                // clk += 2N
                clk
            }

            // ldrh
            1 => {
                let (data, clk) = memory.read16(addr);
                let data = if !addr.is_multiple_of(2) {
                    (data as u32).rotate_right(8)
                } else {
                    data as u32
                };

                self.set_register_value(rd, data);

                // clk += 1S+1N+1I
                clk
            }

            _ => {
                unreachable!();
            }
        }
    }

    fn ls_execute_spr(&mut self, memory: &mut GBAMemory, opcode: u8, rd: usize, imm: u32) -> u32 {
        let addr = self.get_register_value(SP).wrapping_add(imm);
        match opcode {
            // str
            0 => {
                let data = self.get_register_value(rd);

                let clk: u32 = memory.write32(addr, data);

                // clk += 2N
                clk
            }

            // ldr
            1 => {
                let (data, clk) = memory.read32(addr);
                let data = data.rotate_right(8 * (addr & 3));

                self.set_register_value(rd, data);

                // clk += 1S + 1N + 1I
                clk
            }

            _ => {
                unreachable!();
            }
        }
    }
}

// THUMB Memory Addressing
impl CPU {
    fn ma_execute_ra(&mut self, opcode: u8, rd: usize, imm: u32) {
        match opcode {
            // add rd, pc, imm
            0 => {
                let data = ((self.registers.pc + 2) & !2).wrapping_add(imm);

                self.set_register_value(rd, data);
            }

            // add rd, sp, imm
            1 => {
                let data = self.get_register_value(SP).wrapping_add(imm);

                self.set_register_value(rd, data);
            }

            _ => {
                unreachable!();
            }
        }

        // clk += 1S
    }

    fn ma_execute_spo(&mut self, opcode: u8, imm: u32) {
        let sp_value = self.get_register_value(SP);

        let data = match opcode {
            // add sp, imm
            0 => sp_value.wrapping_add(imm),

            // sub sp, imm
            1 => sp_value.wrapping_sub(imm),

            _ => {
                unreachable!();
            }
        };

        self.set_register_value(SP, data);

        // clk += 1S
    }
}

// THUMB Multiple Load Store
impl CPU {
    fn mls_exec_pp(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        pc_lr: u8,
        rlist: &mut Vec<usize>,
    ) {
        match opcode {
            // push
            0 => {
                if pc_lr == 1 {
                    rlist.push(LR);
                }
                for &reg in rlist.iter().rev() {
                    let data = self.get_register_value(reg);
                    let addr = self.get_register_value(SP).wrapping_sub(4);
                    memory.write32(addr, data);

                    self.set_register_value(SP, addr);
                }

                // clk += (n-1)S + 2N
            }
            // pop
            1 => {
                if pc_lr == 1 {
                    rlist.push(PC);
                }

                for &reg in rlist.iter() {
                    let addr = self.get_register_value(SP);
                    let data = if reg == PC {
                        memory.read32(addr).0 & !1
                    } else {
                        memory.read32(addr).0
                    };

                    self.set_register_value(reg, data);

                    self.set_register_value(SP, addr.wrapping_add(4));
                }

                // clk += nS +1N + 1I (+1S +1N if PC is loaded)
            }

            _ => {
                unreachable!();
            }
        }
    }

    fn mls_exec_mls(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        rb: usize,
        rlist: &mut Vec<usize>,
    ) {
        let empty = rlist.is_empty();
        if empty {
            rlist.push(PC);
        }

        match opcode {
            // stmia
            0 => {
                let init_addr = self.get_register_value(rb);
                let mut addr = init_addr;
                let end_base_value = if empty {
                    init_addr.wrapping_add(0x40)
                } else {
                    init_addr.wrapping_add(4 * rlist.len() as u32)
                };

                for (i, &reg) in rlist.iter().enumerate() {
                    let data = if reg == PC {
                        self.registers.pc.wrapping_add(2)
                    } else if reg == rb {
                        if i == 0 {
                            init_addr
                        } else {
                            end_base_value
                        }
                    } else {
                        self.get_register_value(reg)
                    };

                    memory.write32(addr, data);
                    addr = addr.wrapping_add(4);
                }

                self.set_register_value(rb, end_base_value);
            }

            // ldmia
            1 => {
                let init_addr = self.get_register_value(rb);
                let mut addr = init_addr;

                let end_base_value = if empty {
                    init_addr.wrapping_add(0x40)
                } else {
                    init_addr.wrapping_add(4 * rlist.len() as u32)
                };

                for &reg in rlist.iter() {
                    let data = if reg == PC {
                        memory.read32(addr).0 & !1
                    } else {
                        memory.read32(addr).0
                    };
                    self.set_register_value(reg, data);
                    addr = addr.wrapping_add(4);
                }

                if !rlist.contains(&rb) {
                    self.set_register_value(rb, end_base_value);
                }
            }

            _ => {
                unreachable!();
            }
        }
    }
}

// THUMB Jumps and Calls
impl CPU {
    fn jc_execute_cb(&mut self, cond: bool, offset: u32) {
        if !cond {
            return;
            // clk += 1S
        }
        let dest = self.registers.pc.wrapping_add(2).wrapping_add(offset) & !1;
        self.registers.pc = dest;
        // clk += 2S + 1N
    }
}
