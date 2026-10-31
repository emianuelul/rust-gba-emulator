#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::cpu::Cpu;
    use crate::memory_area::GBAMemory;

    const IWRAM: u32 = 0x0300_0000;
    const DATA: u32 = 0x0300_0100;

    fn build(base: u32, program: &[u16], regs: &[(usize, u32)]) -> (Cpu, GBAMemory) {
        let mut mem = GBAMemory::new(vec![0u8; 0x100]);
        for (i, h) in program.iter().enumerate() {
            mem.write16(base + i as u32 * 2, *h);
        }
        let mut cpu = Cpu::new();
        cpu.set_cpsr_bit(T_FLAG, 1);

        for &(r, v) in regs {
            cpu.set_register_value(r, v);
        }
        cpu.registers.pc = base;
        let _ = cpu.get_refill_clk(&mem, base);
        cpu.registers.pc = base;
        (cpu, mem)
    }

    fn step(cpu: &mut Cpu, mem: &mut GBAMemory) -> u32 {
        cpu.step(mem)
    }

    // -----------------------------------------------------------------------
    //  Encoding Helpers
    // -----------------------------------------------------------------------

    // Format 1: Move Shifted Register: 000_oo_nnnnn_sss_ddd
    fn thumb_shift(opcode: u16, offset5: u16, rs: u16, rd: u16) -> u16 {
        (opcode << 11) | ((offset5 & 0x1F) << 6) | ((rs & 7) << 3) | (rd & 7)
    }

    // Format 2: Add / Subtract (3-register or 3-bit imm): 00011_oo_nnn_sss_ddd
    fn thumb_add_sub(opcode: u16, rn_imm: u16, rs: u16, rd: u16) -> u16 {
        (0b00011 << 11) | (opcode << 9) | ((rn_imm & 7) << 6) | ((rs & 7) << 3) | (rd & 7)
    }

    // Format 3: Move / Compare / Add / Subtract Immediate: 001_oo_ddd_nnnnnnnn
    fn thumb_imm(opcode: u16, rd: u16, imm8: u16) -> u16 {
        (0b001 << 13) | (opcode << 11) | ((rd & 7) << 8) | (imm8 & 0xFF)
    }

    // Format 4: ALU Operations: 010000_oooo_sss_ddd
    fn thumb_alu(opcode: u16, rs: u16, rd: u16) -> u16 {
        (0b010000 << 10) | (opcode << 6) | ((rs & 7) << 3) | (rd & 7)
    }

    // Format 5: Hi Register Operations / BX: 010001_oo_a_b_ccc_ddd
    fn thumb_hi_reg(opcode: u16, h1: u16, h2: u16, rs: u16, rd: u16) -> u16 {
        (0b010001 << 10) | (opcode << 8) | (h1 << 7) | (h2 << 6) | ((rs & 7) << 3) | (rd & 7)
    }

    // Format 6: Load PC-Relative: 01001_ddd_nnnnnnnn
    fn thumb_ldr_pc(rd: u16, imm8: u16) -> u16 {
        (0b01001 << 11) | ((rd & 7) << 8) | (imm8 & 0xFF)
    }

    // Format 7: Load / Store with Register Offset: 0101_oo_0_fff_bbb_ddd
    fn thumb_ls_reg(opcode: u16, ro: u16, rb: u16, rd: u16) -> u16 {
        (0b0101 << 12) | (opcode << 10) | (0 << 9) | ((ro & 7) << 6) | ((rb & 7) << 3) | (rd & 7)
    }

    // Format 8: Load / Store Sign-Extended Byte / Halfword: 0101_oo_1_fff_bbb_ddd
    fn thumb_ls_sign_ext(opcode: u16, ro: u16, rb: u16, rd: u16) -> u16 {
        (0b0101 << 12) | (opcode << 10) | (1 << 9) | ((ro & 7) << 6) | ((rb & 7) << 3) | (rd & 7)
    }

    // Format 9: Load / Store with Immediate Offset: 011_oo_nnnnn_bbb_ddd
    fn thumb_ls_imm(opcode: u16, imm5: u16, rb: u16, rd: u16) -> u16 {
        (0b011 << 13) | (opcode << 11) | ((imm5 & 0x1F) << 6) | ((rb & 7) << 3) | (rd & 7)
    }

    // Format 10: Load / Store Halfword: 1000_o_nnnnn_bbb_ddd
    fn thumb_ls_halfword(opcode: u16, imm5: u16, rb: u16, rd: u16) -> u16 {
        (0b1000 << 12) | (opcode << 11) | ((imm5 & 0x1F) << 6) | ((rb & 7) << 3) | (rd & 7)
    }

    // Format 11: Load / Store SP-Relative: 1001_o_ddd_nnnnnnnn
    fn thumb_ls_sp(opcode: u16, rd: u16, imm8: u16) -> u16 {
        (0b1001 << 12) | (opcode << 11) | ((rd & 7) << 8) | (imm8 & 0xFF)
    }

    // Format 12: Get Relative Address: 1010_o_ddd_nnnnnnnn
    fn thumb_rel_addr(opcode: u16, rd: u16, imm8: u16) -> u16 {
        (0b1010 << 12) | (opcode << 11) | ((rd & 7) << 8) | (imm8 & 0xFF)
    }

    // Format 13: Add Offset to Stack Pointer: 10110000_o_nnnnnnn
    fn thumb_add_sp(opcode: u16, imm7: u16) -> u16 {
        (0b10110000 << 8) | (opcode << 7) | (imm7 & 0x7F)
    }

    // Format 14: Push / Pop: 1011_o_10_b_rrrrrrrr
    fn thumb_push_pop(opcode: u16, pc_lr: u16, rlist: u16) -> u16 {
        (0b1011 << 12) | (opcode << 11) | (0b10 << 9) | (pc_lr << 8) | (rlist & 0xFF)
    }

    // Format 15: Multiple Load / Store: 1100_o_bbb_rrrrrrrr
    fn thumb_mls(opcode: u16, rb: u16, rlist: u16) -> u16 {
        (0b1100 << 12) | (opcode << 11) | ((rb & 7) << 8) | (rlist & 0xFF)
    }

    // Format 16: Conditional Branch: 1101_cccc_oooooooo
    fn thumb_cond_branch(cond: u16, offset8: u16) -> u16 {
        (0b1101 << 12) | (cond << 8) | (offset8 & 0xFF)
    }

    // Format 17: Software Interrupt: 11011111_nnnnnnnn
    fn thumb_swi(imm8: u16) -> u16 {
        (0b11011111 << 8) | (imm8 & 0xFF)
    }

    // Format 18: Unconditional Branch: 11100_nnnnnnnnnnn
    fn thumb_b(offset11: u16) -> u16 {
        (0b11100 << 11) | (offset11 & 0x7FF)
    }

    // Format 19: Long Branch with Link 1st half: 11110_nnnnnnnnnnn
    fn thumb_bl_hi(offset11: u16) -> u16 {
        (0b11110 << 11) | (offset11 & 0x7FF)
    }

    // Format 19: Long Branch with Link 2nd half: 11111_nnnnnnnnnnn
    fn thumb_bl_lo(offset11: u16) -> u16 {
        (0b11111 << 11) | (offset11 & 0x7FF)
    }

    // =======================================================================
    //  FORMAT 1: MOVE SHIFTED REGISTER (LSL, LSR, ASR)
    // =======================================================================

    #[test]
    fn thumb_lsl_imm() {
        // LSL R0, R1, #4
        let instr = thumb_shift(0b00, 4, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF0);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn thumb_lsl_imm_zero_offset() {
        // LSL with offset=0: value unchanged, C flag unchanged
        let instr = thumb_shift(0b00, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 42)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_lsl_carry_out() {
        // LSL R0, R1, #1 where bit 31 was 1 => C=1
        let instr = thumb_shift(0b00, 1, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x8000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn thumb_lsr_imm() {
        // LSR R0, R1, #2
        let instr = thumb_shift(0b01, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 16)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 4);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
    }

    #[test]
    fn thumb_lsr_imm_zero_means_32() {
        // In THUMB Format 1, LSR with offset 0 encodes shift of 32
        let instr = thumb_shift(0b01, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x8000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn thumb_asr_positive() {
        // ASR R0, R1, #2 (positive value)
        let instr = thumb_shift(0b10, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x40)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x10);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 0);
    }

    #[test]
    fn thumb_asr_negative_sign_extends() {
        // ASR R0, R1, #4 (negative value, sign extends)
        let instr = thumb_shift(0b10, 4, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xF000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFF00_0000);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    // =======================================================================
    //  FORMAT 2: ADD / SUB (3-register or 3-bit imm)
    // =======================================================================

    #[test]
    fn thumb_add_reg() {
        // ADD R0, R1, R2
        let instr = thumb_add_sub(0b00, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 15), (2, 27)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn thumb_sub_reg() {
        // SUB R0, R1, R2
        let instr = thumb_add_sub(0b01, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 50), (2, 20)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 30);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1); // no borrow
    }

    #[test]
    fn thumb_sub_reg_borrow() {
        let instr = thumb_add_sub(0b01, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 5), (2, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFB);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0); // borrow
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn thumb_add_3bit_imm() {
        // ADD R0, R1, #5
        let instr = thumb_add_sub(0b10, 5, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 105);
    }

    #[test]
    fn thumb_add_3bit_imm_zero_is_mov() {
        // ADD Rd, Rs, #0 acts as MOV Rd, Rs
        let instr = thumb_add_sub(0b10, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x1234)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x1234);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 0);
    }

    #[test]
    fn thumb_sub_3bit_imm() {
        // SUB R0, R1, #3
        let instr = thumb_add_sub(0b11, 3, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 7);
    }

    // =======================================================================
    //  FORMAT 3: MOVE / COMPARE / ADD / SUB IMMEDIATE (8-bit)
    // =======================================================================

    #[test]
    fn thumb_mov_imm8() {
        // MOV R0, #255
        let instr = thumb_imm(0b00, 0, 255);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 255);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn thumb_mov_imm8_zero() {
        let instr = thumb_imm(0b00, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn thumb_cmp_imm8_equal() {
        // CMP R0, #42 where R0=42
        let instr = thumb_imm(0b01, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 42)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42); // Rd preserved
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_cmp_imm8_less() {
        // CMP R0, #50 where R0=10 => underflow, C=0, N=1
        let instr = thumb_imm(0b01, 0, 50);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn thumb_add_imm8() {
        // ADD R0, #100
        let instr = thumb_imm(0b10, 0, 100);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 50)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 150);
    }

    #[test]
    fn thumb_add_imm8_carry() {
        let instr = thumb_imm(0b10, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn thumb_sub_imm8() {
        // SUB R0, #25
        let instr = thumb_imm(0b11, 0, 25);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 75)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 50);
    }

    // =======================================================================
    //  FORMAT 4: ALU OPERATIONS (ALL 16 OPCODES)
    // =======================================================================

    #[test]
    fn thumb_alu_and() {
        let instr = thumb_alu(0x0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFF00), (1, 0x0FF0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x0F00);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn thumb_alu_eor() {
        let instr = thumb_alu(0x1, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xAAAA), (1, 0x5555)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF);
    }

    #[test]
    fn thumb_alu_lsl() {
        // LSL Rd, Rs (shift amount from Rs & 0xFF)
        let instr = thumb_alu(0x2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x01), (1, 8)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x0100);
    }

    #[test]
    fn thumb_alu_lsl_ge_32() {
        let instr = thumb_alu(0x2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x01), (1, 32)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_alu_lsr() {
        let instr = thumb_alu(0x3, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x0400), (1, 8)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 4);
    }

    #[test]
    fn thumb_alu_asr() {
        let instr = thumb_alu(0x4, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x8000_0000), (1, 4)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF800_0000);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn thumb_alu_adc() {
        // ADC Rd, Rs
        let instr = thumb_alu(0x5, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 10), (1, 20)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 31);
    }

    #[test]
    fn thumb_alu_sbc() {
        // SBC Rd, Rs with C=1 (no borrow subtracted)
        let instr = thumb_alu(0x6, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 30), (1, 10)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 20);
    }

    #[test]
    fn thumb_alu_sbc_borrow() {
        // SBC Rd, Rs with C=0 (borrow subtracted)
        let instr = thumb_alu(0x6, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 30), (1, 10)]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 19);
    }

    #[test]
    fn thumb_alu_ror() {
        let instr = thumb_alu(0x7, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x0F), (1, 4)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF000_0000);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_alu_tst() {
        let instr = thumb_alu(0x8, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xF0), (1, 0x0F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF0); // unchanged
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn thumb_alu_neg() {
        // NEG Rd, Rs => Rd = 0 - Rs
        let instr = thumb_alu(0x9, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFF6);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn thumb_alu_cmp() {
        let instr = thumb_alu(0xA, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 100), (1, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 100); // unchanged
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_alu_cmn() {
        // CMN Rd, Rs => Rd + Rs
        let instr = thumb_alu(0xB, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFFFF_FFFF), (1, 1)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF); // unchanged
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_alu_orr() {
        let instr = thumb_alu(0xC, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x0F0F), (1, 0xF0F0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF);
    }

    #[test]
    fn thumb_alu_mul() {
        let instr = thumb_alu(0xD, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 6), (1, 7)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
    }

    #[test]
    fn thumb_alu_bic() {
        let instr = thumb_alu(0xE, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFFFF), (1, 0x00FF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFF00);
    }

    #[test]
    fn thumb_alu_mvn() {
        let instr = thumb_alu(0xF, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    // =======================================================================
    //  FORMAT 5: HI REGISTER OPERATIONS / BRANCH EXCHANGE
    // =======================================================================

    #[test]
    fn thumb_hi_reg_add() {
        // ADD R8, R1 (h1=1, h2=0)
        let instr = thumb_hi_reg(0b00, 1, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(8, 100), (1, 50)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(8), 150);
    }

    #[test]
    fn thumb_hi_reg_add_pc() {
        // ADD PC, R0 (branches forward)
        let instr = thumb_hi_reg(0b00, 1, 0, 0, 7); // Rd=15 (PC)
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 4)]);
        step(&mut cpu, &mut mem);
        // PC should be updated with pipeline refill
        assert!(cpu.get_register_value(PC) > IWRAM);
    }

    #[test]
    fn thumb_hi_reg_cmp() {
        // CMP R8, R9 (h1=1, h2=1)
        let instr = thumb_hi_reg(0b01, 1, 1, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(8, 42), (9, 42)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn thumb_hi_reg_mov() {
        // MOV R8, R1
        let instr = thumb_hi_reg(0b10, 1, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xCAFE)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(8), 0xCAFE);
    }

    #[test]
    fn thumb_hi_reg_mov_pc() {
        // MOV PC, R1 (jump)
        let dest = IWRAM + 0x40;
        let instr = thumb_hi_reg(0b10, 1, 0, 1, 7); // Rd=PC (15)
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, dest | 1)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), dest);
    }

    #[test]
    fn thumb_bx_stay_thumb() {
        // BX R0 where R0 has bit 0 set => stay in THUMB mode
        let dest = IWRAM + 0x41;
        let instr = thumb_hi_reg(0b11, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, dest)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(T_FLAG), 1);
        assert_eq!(cpu.get_register_value(PC), dest & !1);
    }

    #[test]
    fn thumb_bx_switch_to_arm() {
        // BX R0 where R0 has bit 0 cleared => switch to ARM mode
        let dest = IWRAM + 0x40;
        let instr = thumb_hi_reg(0b11, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, dest)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(T_FLAG), 0);
        assert_eq!(cpu.get_register_value(PC), (dest + 2) & !3);
    }

    // =======================================================================
    //  FORMAT 6: LOAD PC-RELATIVE (LITERAL POOL)
    // =======================================================================

    #[test]
    fn thumb_ldr_pc_relative() {
        // LDR R0, [PC, #4]
        let instr = thumb_ldr_pc(0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        // pc_value = (pc + 2) & !2 = (IWRAM + 2) & !2 = IWRAM
        // wait: after fetch, pc is IWRAM + 2. In ls_execute_pcr:
        // pc_value = (pc + 2) & !2 = (IWRAM + 4) & !2 = IWRAM + 4
        // addr = pc_value + (1 << 2) = IWRAM + 8
        mem.write32(IWRAM + 8, 0x1234_5678);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x1234_5678);
    }

    // =======================================================================
    //  FORMAT 7: LOAD / STORE WITH REGISTER OFFSET
    // =======================================================================

    #[test]
    fn thumb_str_reg_offset() {
        // STR R0, [R1, R2]
        let instr = thumb_ls_reg(0b00, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xDEAD_BEEF), (1, DATA), (2, 8)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA + 8), 0xDEAD_BEEF);
    }

    #[test]
    fn thumb_strb_reg_offset() {
        // STRB R0, [R1, R2]
        let instr = thumb_ls_reg(0b01, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xAB), (1, DATA), (2, 3)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read8(DATA + 3).0, 0xAB);
    }

    #[test]
    fn thumb_ldr_reg_offset() {
        // LDR R0, [R1, R2]
        let instr = thumb_ls_reg(0b10, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 4)]);
        mem.write32(DATA + 4, 0xCAFE_BABE);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xCAFE_BABE);
    }

    #[test]
    fn thumb_ldrb_reg_offset() {
        // LDRB R0, [R1, R2]
        let instr = thumb_ls_reg(0b11, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 1)]);
        mem.write8(DATA + 1, 0xCD);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xCD);
    }

    // =======================================================================
    //  FORMAT 8: LOAD / STORE SIGN-EXTENDED BYTE / HALFWORD
    // =======================================================================

    #[test]
    fn thumb_strh_reg_offset() {
        let instr = thumb_ls_sign_ext(0b00, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x1234), (1, DATA), (2, 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read16(DATA + 2).0, 0x1234);
    }

    #[test]
    fn thumb_ldsb_reg_offset() {
        let instr = thumb_ls_sign_ext(0b01, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0)]);
        mem.write8(DATA, 0x80); // -128 signed
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FF80);
    }

    #[test]
    fn thumb_ldrh_reg_offset() {
        let instr = thumb_ls_sign_ext(0b10, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 2)]);
        mem.write16(DATA + 2, 0xBEEF);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xBEEF);
    }

    #[test]
    fn thumb_ldsh_reg_offset() {
        let instr = thumb_ls_sign_ext(0b11, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0)]);
        mem.write16(DATA, 0x8000); // -32768 signed
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_8000);
    }

    #[test]
    fn thumb_ldsh_reg_offset_positive() {
        let instr = thumb_ls_sign_ext(0b11, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0)]);
        mem.write16(DATA, 0x7FFF);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7FFF);
    }

    // =======================================================================
    //  FORMAT 9: LOAD / STORE WITH IMMEDIATE OFFSET
    // =======================================================================

    #[test]
    fn thumb_str_imm_offset() {
        // STR R0, [R1, #8] (imm5=2 => 2*4=8)
        let instr = thumb_ls_imm(0b00, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x5555_AAAA), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA + 8), 0x5555_AAAA);
    }

    #[test]
    fn thumb_ldr_imm_offset() {
        // LDR R0, [R1, #4] (imm5=1 => 1*4=4)
        let instr = thumb_ls_imm(0b01, 1, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA + 4, 0x3333_4444);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x3333_4444);
    }

    #[test]
    fn thumb_strb_imm_offset() {
        // STRB R0, [R1, #3] (imm5=3)
        let instr = thumb_ls_imm(0b10, 3, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x77), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read8(DATA + 3).0, 0x77);
    }

    #[test]
    fn thumb_ldrb_imm_offset() {
        // LDRB R0, [R1, #5]
        let instr = thumb_ls_imm(0b11, 5, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write8(DATA + 5, 0x99);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x99);
    }

    // =======================================================================
    //  FORMAT 10: LOAD / STORE HALFWORD IMMEDIATE
    // =======================================================================

    #[test]
    fn thumb_strh_imm_offset() {
        // STRH R0, [R1, #4] (imm5=2 => 2*2=4)
        let instr = thumb_ls_halfword(0, 2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x4321), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read16(DATA + 4).0, 0x4321);
    }

    #[test]
    fn thumb_ldrh_imm_offset() {
        // LDRH R0, [R1, #6] (imm5=3 => 3*2=6)
        let instr = thumb_ls_halfword(1, 3, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write16(DATA + 6, 0x8765);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x8765);
    }

    // =======================================================================
    //  FORMAT 11: LOAD / STORE SP-RELATIVE
    // =======================================================================

    #[test]
    fn thumb_str_sp_relative() {
        // STR R0, [SP, #8] (imm8=2 => 2*4=8)
        let instr = thumb_ls_sp(0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xAAAA_1111), (SP, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA + 8), 0xAAAA_1111);
    }

    #[test]
    fn thumb_ldr_sp_relative() {
        // LDR R0, [SP, #12] (imm8=3 => 3*4=12)
        let instr = thumb_ls_sp(1, 0, 3);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA)]);
        mem.write32(DATA + 12, 0xBBBB_2222);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xBBBB_2222);
    }

    // =======================================================================
    //  FORMAT 12: GET RELATIVE ADDRESS
    // =======================================================================

    #[test]
    fn thumb_add_pc_relative() {
        // ADD R0, PC, #16 (imm8=4 => 4*4=16)
        let instr = thumb_rel_addr(0, 0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        // PC after fetch = IWRAM + 2. pc_val = (pc + 2) & !2 = (IWRAM + 4) & !2 = IWRAM + 4
        // Result = (IWRAM + 4) + 16 = IWRAM + 20
        assert_eq!(cpu.get_register_value(0), IWRAM + 20);
    }

    #[test]
    fn thumb_add_sp_relative() {
        // ADD R0, SP, #20 (imm8=5 => 5*4=20)
        let instr = thumb_rel_addr(1, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), DATA + 20);
    }

    // =======================================================================
    //  FORMAT 13: ADD OFFSET TO STACK POINTER
    // =======================================================================

    #[test]
    fn thumb_add_sp_offset() {
        // ADD SP, #16 (imm7=4 => 4*4=16)
        let instr = thumb_add_sp(0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(SP), DATA + 16);
    }

    #[test]
    fn thumb_sub_sp_offset() {
        // SUB SP, #16 (imm7=4 => 4*4=16)
        let instr = thumb_add_sp(1, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA + 32)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(SP), DATA + 16);
    }

    // =======================================================================
    //  FORMAT 14: PUSH / POP REGISTERS
    // =======================================================================

    #[test]
    fn thumb_push_regs_and_lr() {
        // PUSH {R0, R1, LR}
        let instr = thumb_push_pop(0, 1, 0b0000_0011);
        let (mut cpu, mut mem) = build(
            IWRAM,
            &[instr],
            &[(SP, DATA + 32), (0, 0x11), (1, 0x22), (LR, 0x33)],
        );
        step(&mut cpu, &mut mem);
        // SP moves down by 3*4 = 12
        assert_eq!(cpu.get_register_value(SP), DATA + 20);
        assert_eq!(mem.peek32(DATA + 20), 0x11);
        assert_eq!(mem.peek32(DATA + 24), 0x22);
        assert_eq!(mem.peek32(DATA + 28), 0x33);
    }

    #[test]
    fn thumb_pop_regs_and_pc() {
        // POP {R0, R1, PC}
        let dest = IWRAM + 0x40;
        let instr = thumb_push_pop(1, 1, 0b0000_0011);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA)]);
        mem.write32(DATA, 0xAA);
        mem.write32(DATA + 4, 0xBB);
        mem.write32(DATA + 8, dest | 1);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(0), 0xAA);
        assert_eq!(cpu.get_register_value(1), 0xBB);
        assert_eq!(cpu.get_register_value(PC), dest);
        assert_eq!(cpu.get_register_value(SP), DATA + 12);
    }

    // =======================================================================
    //  FORMAT 15: MULTIPLE LOAD / STORE (LDMIA / STMIA)
    // =======================================================================

    #[test]
    fn thumb_stmia() {
        // STMIA R0!, {R1, R2}
        let instr = thumb_mls(0, 0, (1 << 1) | (1 << 2));
        let (mut cpu, mut mem) =
            build(IWRAM, &[instr], &[(0, DATA), (1, 0x1234), (2, 0x5678)]);
        step(&mut cpu, &mut mem);

        assert_eq!(mem.peek32(DATA), 0x1234);
        assert_eq!(mem.peek32(DATA + 4), 0x5678);
        assert_eq!(cpu.get_register_value(0), DATA + 8);
    }

    #[test]
    fn thumb_ldmia() {
        // LDMIA R0!, {R1, R2}
        let instr = thumb_mls(1, 0, (1 << 1) | (1 << 2));
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA)]);
        mem.write32(DATA, 0x7777);
        mem.write32(DATA + 4, 0x8888);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(1), 0x7777);
        assert_eq!(cpu.get_register_value(2), 0x8888);
        assert_eq!(cpu.get_register_value(0), DATA + 8);
    }

    // =======================================================================
    //  FORMAT 16: CONDITIONAL BRANCH
    // =======================================================================

    #[test]
    fn thumb_cond_branch_eq_taken() {
        // BEQ forward +4 bytes (offset = +2 halfwords = 2)
        let instr = thumb_cond_branch(0x0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        step(&mut cpu, &mut mem);
        // dest = (pc + 2 + (offset8 as i8 as i32 << 1)) & !1
        // dest = (IWRAM + 2 + 2 + 4) & !1 = IWRAM + 8
        assert_eq!(cpu.get_register_value(PC), IWRAM + 8);
    }

    #[test]
    fn thumb_cond_branch_eq_not_taken() {
        let instr = thumb_cond_branch(0x0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        step(&mut cpu, &mut mem);
        // PC advances naturally to IWRAM + 2
        assert_eq!(cpu.get_register_value(PC), IWRAM + 2);
    }

    #[test]
    fn thumb_cond_branch_ne_taken() {
        let instr = thumb_cond_branch(0x1, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM + 6);
    }

    #[test]
    fn thumb_cond_branch_backward() {
        // B self: offset8 = 0xFE (-2 halfwords = -4 bytes)
        // dest = (pc + 2) + offset = (IWRAM + 4) - 4 = IWRAM
        let instr = thumb_cond_branch(0xE, 0xFE); // AL condition code
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM);
    }

    // =======================================================================
    //  FORMAT 17: SWI
    // =======================================================================

    #[test]
    fn thumb_swi_executes() {
        let instr = thumb_swi(0x12);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        let clk = step(&mut cpu, &mut mem);
        assert_eq!(clk, 1);
    }

    // =======================================================================
    //  FORMAT 18: UNCONDITIONAL BRANCH
    // =======================================================================

    #[test]
    fn thumb_unconditional_branch_forward() {
        // B forward +4 bytes (offset11 = 2)
        let instr = thumb_b(2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM + 8);
    }

    #[test]
    fn thumb_unconditional_branch_backward() {
        // B self: offset11 = -2 (0x7FE)
        let instr = thumb_b(0x7FE);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM);
    }

    // =======================================================================
    //  FORMAT 19: LONG BRANCH WITH LINK (BL)
    // =======================================================================

    #[test]
    fn thumb_bl_sequence() {
        // BL hi + BL lo
        let hi = thumb_bl_hi(0);
        let lo = thumb_bl_lo(4); // target offset + 8
        let (mut cpu, mut mem) = build(IWRAM, &[hi, lo], &[]);
        step(&mut cpu, &mut mem); // first half sets LR
        step(&mut cpu, &mut mem); // second half jumps and sets LR
        assert_eq!(cpu.get_register_value(LR), (IWRAM + 4) | 1);
    }

    // =======================================================================
    //  CYCLE TIMING TESTS (THUMB)
    // =======================================================================

    #[test]
    fn thumb_alu_imm_clk() {
        let instr = thumb_imm(0b00, 0, 42); // MOV R0, #42
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        assert_eq!(step(&mut cpu, &mut mem), 1);
    }

    #[test]
    fn thumb_alu_shift_by_reg_clk() {
        // Format 4 LSL Rd, Rs: 1S + 1I = 2
        let instr = thumb_alu(0x2, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 1), (1, 2)]);
        assert_eq!(step(&mut cpu, &mut mem), 2);
    }

    #[test]
    fn thumb_alu_ror_clk() {
        // Format 4 ROR Rd, Rs: 1S + 1I = 2
        let instr = thumb_alu(0x7, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x10), (1, 1)]);
        assert_eq!(step(&mut cpu, &mut mem), 2);
    }

    #[test]
    fn thumb_branch_clk() {
        // B self: 2S + 1N = 3
        let instr = thumb_b(0x7FE);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        assert_eq!(step(&mut cpu, &mut mem), 3);
    }

    #[test]
    fn thumb_cond_branch_taken_clk() {
        let instr = thumb_cond_branch(0x0, 0xFE); // BEQ self
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        assert_eq!(step(&mut cpu, &mut mem), 3);
    }

    #[test]
    fn thumb_cond_branch_not_taken_clk() {
        let instr = thumb_cond_branch(0x0, 0xFE); // BEQ self
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        assert_eq!(step(&mut cpu, &mut mem), 1); // 1S only
    }

    #[test]
    fn thumb_bx_clk() {
        let instr = thumb_hi_reg(0b11, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, IWRAM + 0x21)]);
        assert_eq!(step(&mut cpu, &mut mem), 3);
    }

    #[test]
    fn thumb_ldr_pcr_clk() {
        let instr = thumb_ldr_pc(0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1N + 1I
    }

    #[test]
    fn thumb_str_clk() {
        let instr = thumb_ls_imm(0b00, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0), (1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 1S + 1N
    }

    #[test]
    fn thumb_ldr_clk() {
        let instr = thumb_ls_imm(0b01, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1N + 1I
    }

    #[test]
    fn thumb_strh_clk() {
        let instr = thumb_ls_halfword(0, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0), (1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 2);
    }

    #[test]
    fn thumb_ldrh_clk() {
        let instr = thumb_ls_halfword(1, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 3);
    }

    #[test]
    fn thumb_push_clk() {
        let instr = thumb_push_pop(0, 0, 1 << 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA + 16)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 2N for 1 write
    }

    #[test]
    fn thumb_pop_pc_clk() {
        let instr = thumb_push_pop(1, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(SP, DATA)]);
        mem.write32(DATA, (IWRAM + 0x40) | 1);
        assert_eq!(step(&mut cpu, &mut mem), 5); // 2S + 2N + 1I
    }

    #[test]
    fn thumb_mul_clk() {
        let instr = thumb_alu(0xD, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFF), (1, 3)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 1S + 1
    }

    // =======================================================================
    //  EDGE CASES
    // =======================================================================

    #[test]
    fn thumb_add_wrapping() {
        let instr = thumb_imm(0b10, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    #[test]
    fn thumb_sub_wrapping() {
        let instr = thumb_imm(0b11, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
    }

    #[test]
    fn thumb_ldr_unaligned_rotates() {
        // LDR from unaligned address rotates data
        let instr = thumb_ls_imm(0b01, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA + 2)]);
        mem.write32(DATA, 0x04030201);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x02010403);
    }

    #[test]
    fn thumb_str_unaligned_force_aligns() {
        let instr = thumb_ls_imm(0b00, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xCAFE_BABE), (1, DATA + 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA), 0xCAFE_BABE);
    }
}
