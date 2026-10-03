use crate::constants::*;
use crate::cpu_module::*;
use crate::memory_area::*;

// THUMB Register Op Logic
impl CPU {
    pub fn ro_execute_move_shifted(&mut self, opcode: u8, rd: usize, rs: usize, offset: u8) {
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

    pub fn ro_execute_add_sub(&mut self, opcode: u8, rd: usize, rs: usize, operand: u8) {
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

    pub fn ro_execute_mcas_op(&mut self, opcode: u8, rd: usize, imm: u8) {
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

    pub fn ro_execute_alu_op(&mut self, opcode: u8, rs: usize, rd: usize) {
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

    pub fn ro_execute_hi_reg(
        &mut self,
        opcode: u8,
        msbd: usize,
        msbs: usize,
        rs: usize,
        rd: usize,
    ) {
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
    pub fn ls_execute_pcr(&mut self, memory: &mut GBAMemory, rd: usize, imm: u32) -> u32 {
        let pc_value = (self.registers.pc.wrapping_add(2)) & !2;

        let addr = pc_value.wrapping_add(imm);
        let (data, clk) = memory.read32(addr);

        self.set_register_value(rd, data);

        // clk += 1S + 1N + 1I
        clk
    }

    pub fn ls_execute_ro(
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

    pub fn ls_execute_sh(
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

    pub fn ls_execute_io(
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

    pub fn ls_execute_h(
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

    pub fn ls_execute_spr(
        &mut self,
        memory: &mut GBAMemory,
        opcode: u8,
        rd: usize,
        imm: u32,
    ) -> u32 {
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
    pub fn ma_execute_ra(&mut self, opcode: u8, rd: usize, imm: u32) {
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

    pub fn ma_execute_spo(&mut self, opcode: u8, imm: u32) {
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
    pub fn mls_exec_pp(
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

    pub fn mls_exec_mls(
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
    pub fn jc_execute_cb(&mut self, cond: bool, offset: u32) {
        if !cond {
            return;
            // clk += 1S
        }
        let dest = self.registers.pc.wrapping_add(2).wrapping_add(offset) & !1;
        self.registers.pc = dest;
        // clk += 2S + 1N
    }
}
