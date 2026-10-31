#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::cpu::{CPUMode, Cpu};
    use crate::memory_area::GBAMemory;

    const IWRAM: u32 = 0x0300_0000;
    const DATA: u32 = 0x0300_0100;

    const AL: u32 = 0xE; // always condition code

    fn build(base: u32, program: &[u32], regs: &[(usize, u32)]) -> (Cpu, GBAMemory) {
        let mut mem = GBAMemory::new(vec![0u8; 0x100]);
        for (i, w) in program.iter().enumerate() {
            mem.write32(base + i as u32 * 4, *w);
        }
        let mut cpu = Cpu::new();

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

    // ALU immediate: cccc 001 oooo s rrrr dddd hhhh nnnnnnnn
    fn alu_imm(cond: u32, opcode: u32, s: u32, rn: u32, rd: u32, rotate: u32, imm: u32) -> u32 {
        (cond << 28)
            | (0b001 << 25)
            | (opcode << 21)
            | (s << 20)
            | (rn << 16)
            | (rd << 12)
            | (rotate << 8)
            | (imm & 0xFF)
    }

    // ALU reg shifted by imm: cccc 000 oooo s rrrr dddd hhhhh tt 0 mmmm
    fn alu_reg(
        cond: u32,
        opcode: u32,
        s: u32,
        rn: u32,
        rd: u32,
        shift_amount: u32,
        shift_type: u32,
        rm: u32,
    ) -> u32 {
        (cond << 28)
            | (0b000 << 25)
            | (opcode << 21)
            | (s << 20)
            | (rn << 16)
            | (rd << 12)
            | (shift_amount << 7)
            | (shift_type << 5)
            | rm
    }

    // ALU reg shifted by reg: cccc 000 oooo s rrrr dddd hhhh 0 tt 1 mmmm
    fn alu_reg_shift_by_reg(
        cond: u32,
        opcode: u32,
        s: u32,
        rn: u32,
        rd: u32,
        rs: u32,
        shift_type: u32,
        rm: u32,
    ) -> u32 {
        (cond << 28)
            | (0b000 << 25)
            | (opcode << 21)
            | (s << 20)
            | (rn << 16)
            | (rd << 12)
            | (rs << 8)
            | (shift_type << 5)
            | (1 << 4)
            | rm
    }

    // Branch / Branch with Link: cccc 101 L nnnnnnnnnnnnnnnnnnnnnnnn
    fn branch(cond: u32, link: u32, offset_24: u32) -> u32 {
        (cond << 28) | (0b101 << 25) | (link << 24) | (offset_24 & 0x00FF_FFFF)
    }

    // Branch and Exchange: cccc 0001 0010 1111 1111 1111 0001 rn
    fn bx(cond: u32, rn: u32) -> u32 {
        (cond << 28) | 0x012F_FF10 | rn
    }

    // SWI: cccc 1111 nnnnnnnnnnnnnnnnnnnnnnnn
    fn swi(cond: u32, comment: u32) -> u32 {
        (cond << 28) | (0b1111 << 24) | (comment & 0x00FF_FFFF)
    }

    // Multiply / Multiply-Accumulate: cccc 000 oooo s dddd nnnn ffff 1001 mmmm
    fn mul_op(cond: u32, opcode: u32, s: u32, rd: u32, rn: u32, rs: u32, rm: u32) -> u32 {
        (cond << 28) | (opcode << 21) | (s << 20) | (rd << 16) | (rn << 12) | (rs << 8) | 0x90 | rm
    }

    // SDT immediate: cccc 010 p u b w l rrrr dddd iiiiiiiiiiii
    fn sdt_imm(
        cond: u32,
        p: u32,
        u_bit: u32,
        b: u32,
        w: u32,
        l: u32,
        rn: u32,
        rd: u32,
        offset: u32,
    ) -> u32 {
        (cond << 28)
            | (0b010 << 25)
            | (p << 24)
            | (u_bit << 23)
            | (b << 22)
            | (w << 21)
            | (l << 20)
            | (rn << 16)
            | (rd << 12)
            | (offset & 0xFFF)
    }

    // SDT register shifted: cccc 011 p u b w l rrrr dddd iiiii ss 0 mmmm
    fn sdt_reg(
        cond: u32,
        p: u32,
        u_bit: u32,
        b: u32,
        w: u32,
        l: u32,
        rn: u32,
        rd: u32,
        shift_amount: u32,
        shift_type: u32,
        rm: u32,
    ) -> u32 {
        (cond << 28)
            | (0b011 << 25)
            | (p << 24)
            | (u_bit << 23)
            | (b << 22)
            | (w << 21)
            | (l << 20)
            | (rn << 16)
            | (rd << 12)
            | (shift_amount << 7)
            | (shift_type << 5)
            | rm
    }

    // HSDT immediate: cccc 000 p u 1 w l nnnn dddd aaaa 1 oo 1 bbbb
    fn hsdt_imm(
        cond: u32,
        p: u32,
        u_bit: u32,
        w: u32,
        l: u32,
        rn: u32,
        rd: u32,
        opcode: u32,
        offset: u32,
    ) -> u32 {
        let hi = (offset >> 4) & 0xF;
        let lo = offset & 0xF;
        (cond << 28)
            | (p << 24)
            | (u_bit << 23)
            | (1 << 22)
            | (w << 21)
            | (l << 20)
            | (rn << 16)
            | (rd << 12)
            | (hi << 8)
            | (1 << 7)
            | (opcode << 5)
            | (1 << 4)
            | lo
    }

    // HSDT register offset: cccc 000 p u 0 w l nnnn dddd 0000 1 oo 1 bbbb
    fn hsdt_reg(
        cond: u32,
        p: u32,
        u_bit: u32,
        w: u32,
        l: u32,
        rn: u32,
        rd: u32,
        opcode: u32,
        rm: u32,
    ) -> u32 {
        (cond << 28)
            | (p << 24)
            | (u_bit << 23)
            | (0 << 22)
            | (w << 21)
            | (l << 20)
            | (rn << 16)
            | (rd << 12)
            | (1 << 7)
            | (opcode << 5)
            | (1 << 4)
            | rm
    }

    // BDT (LDM / STM): cccc 100 p u s w l nnnn rrrrrrrrrrrrrrrr
    fn bdt(cond: u32, p: u32, u_bit: u32, s: u32, w: u32, l: u32, rn: u32, rlist: u32) -> u32 {
        (cond << 28)
            | (0b100 << 25)
            | (p << 24)
            | (u_bit << 23)
            | (s << 22)
            | (w << 21)
            | (l << 20)
            | (rn << 16)
            | (rlist & 0xFFFF)
    }

    // SWP: cccc 00010 b 00 nnnn dddd 00001001 mmmm
    fn swp(cond: u32, b: u32, rn: u32, rd: u32, rm: u32) -> u32 {
        (cond << 28) | (0b00010 << 23) | (b << 22) | (rn << 16) | (rd << 12) | 0x90 | rm
    }

    // MRS: cccc 00 0 10 p 0 0 1111 dddd 000000000000
    fn mrs(cond: u32, p: u32, rd: u32) -> u32 {
        (cond << 28) | (0b00010 << 23) | (p << 22) | (0xF << 16) | (rd << 12)
    }

    // MSR (register): cccc 00 0 10 p 1 0 f s x c 1111 00000000 mmmm
    fn msr_reg(cond: u32, p: u32, f: u32, s: u32, x: u32, c: u32, rm: u32) -> u32 {
        (cond << 28)
            | (0b00010 << 23)
            | (p << 22)
            | (0b10 << 20)
            | (f << 19)
            | (s << 18)
            | (x << 17)
            | (c << 16)
            | (0xF << 12)
            | rm
    }

    // MSR (immediate): cccc 00 1 10 p 1 0 f s x c 1111 hhhh iiiiiiii
    fn msr_imm(
        cond: u32,
        p: u32,
        f: u32,
        s: u32,
        x: u32,
        c: u32,
        rotate: u32,
        imm: u32,
    ) -> u32 {
        (cond << 28)
            | (0b00110 << 23)
            | (p << 22)
            | (0b10 << 20)
            | (f << 19)
            | (s << 18)
            | (x << 17)
            | (c << 16)
            | (0xF << 12)
            | (rotate << 8)
            | (imm & 0xFF)
    }

    // =======================================================================
    //  B / BL TESTS
    // =======================================================================

    #[test]
    fn arm_b_forward() {
        // B +8 (offset=1 word relative to PC+8)
        let instr = branch(AL, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM + 12);
    }

    #[test]
    fn arm_b_backward() {
        // B self (-2 words in 24-bit sign-extended format)
        let instr = branch(AL, 0, 0x00FF_FFFE);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), IWRAM);
    }

    #[test]
    fn arm_bl_sets_lr() {
        let instr = branch(AL, 1, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(LR), IWRAM + 4);
    }

    // =======================================================================
    //  BX TESTS
    // =======================================================================

    #[test]
    fn arm_bx_stays_arm() {
        let dest = IWRAM + 0x40;
        let instr = bx(AL, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, dest)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(T_FLAG), 0);
        assert_eq!(cpu.get_register_value(PC), dest & !3);
    }

    #[test]
    fn arm_bx_switches_to_thumb() {
        let dest = IWRAM + 0x41; // odd bit => THUMB
        let instr = bx(AL, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, dest)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(T_FLAG), 1);
        assert_eq!(cpu.get_register_value(PC), dest & !1);
    }

    #[test]
    fn arm_bx_pc() {
        // BX R15
        let instr = bx(AL, 15);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        let pc_before = cpu.registers.pc;
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(T_FLAG), 0);
        assert!(cpu.get_register_value(PC) >= pc_before);
    }

    // =======================================================================
    //  SWI TEST
    // =======================================================================

    #[test]
    fn arm_swi_executes() {
        let instr = swi(AL, 0x1234);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        let clk = step(&mut cpu, &mut mem);
        assert_eq!(clk, 1);
    }

    // =======================================================================
    //  CONDITION CODE TESTS
    // =======================================================================

    #[test]
    fn arm_condition_eq_taken() {
        let instr = alu_imm(0x0, 0xD, 0, 0, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
    }

    #[test]
    fn arm_condition_eq_not_taken() {
        let instr = alu_imm(0x0, 0xD, 0, 0, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0)]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    #[test]
    fn arm_condition_ne() {
        let instr = alu_imm(0x1, 0xD, 0, 0, 0, 0, 99);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 99);
    }

    #[test]
    fn arm_condition_cs() {
        let instr = alu_imm(0x2, 0xD, 0, 0, 0, 0, 77);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 77);
    }

    #[test]
    fn arm_condition_cc() {
        let instr = alu_imm(0x3, 0xD, 0, 0, 0, 0, 55);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 55);
    }

    #[test]
    fn arm_condition_mi() {
        let instr = alu_imm(0x4, 0xD, 0, 0, 0, 0, 11);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(N_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 11);
    }

    #[test]
    fn arm_condition_pl() {
        let instr = alu_imm(0x5, 0xD, 0, 0, 0, 0, 22);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(N_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 22);
    }

    #[test]
    fn arm_condition_vs() {
        let instr = alu_imm(0x6, 0xD, 0, 0, 0, 0, 33);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(V_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 33);
    }

    #[test]
    fn arm_condition_vc() {
        let instr = alu_imm(0x7, 0xD, 0, 0, 0, 0, 44);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(V_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 44);
    }

    #[test]
    fn arm_condition_hi() {
        let instr = alu_imm(0x8, 0xD, 0, 0, 0, 0, 66);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 66);
    }

    #[test]
    fn arm_condition_ls() {
        let instr = alu_imm(0x9, 0xD, 0, 0, 0, 0, 77);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 77);
    }

    #[test]
    fn arm_condition_ge() {
        let instr = alu_imm(0xA, 0xD, 0, 0, 0, 0, 88);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(N_FLAG, 0);
        cpu.set_cpsr_bit(V_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 88);
    }

    #[test]
    fn arm_condition_lt() {
        let instr = alu_imm(0xB, 0xD, 0, 0, 0, 0, 99);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(N_FLAG, 1);
        cpu.set_cpsr_bit(V_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 99);
    }

    #[test]
    fn arm_condition_gt() {
        let instr = alu_imm(0xC, 0xD, 0, 0, 0, 0, 100);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        cpu.set_cpsr_bit(N_FLAG, 0);
        cpu.set_cpsr_bit(V_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 100);
    }

    #[test]
    fn arm_condition_le() {
        let instr = alu_imm(0xD, 0xD, 0, 0, 0, 0, 111);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 111);
    }

    #[test]
    fn arm_condition_nv() {
        let instr = alu_imm(0xF, 0xD, 0, 0, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    // =======================================================================
    //  ALU — ALL 16 OPCODES (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC,
    //                        TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
    // =======================================================================

    #[test]
    fn arm_and_imm() {
        let instr = alu_imm(AL, 0x0, 0, 1, 0, 0, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xDEAD_BEEF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xDEAD_BEEF & 0xFF);
    }

    #[test]
    fn arm_ands_sets_zero_flag() {
        let instr = alu_imm(AL, 0x0, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x1234)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 0);
    }

    #[test]
    fn arm_ands_sets_negative_flag() {
        let instr = alu_reg(AL, 0x0, 1, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF), (2, 0x8000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x8000_0000);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn arm_eor_imm() {
        let instr = alu_imm(AL, 0x1, 0, 1, 0, 0, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x00FF_00FF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x00FF_00FF ^ 0xFF);
    }

    #[test]
    fn arm_eors_zero_flag() {
        let instr = alu_reg(AL, 0x1, 1, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xAAAA), (2, 0xAAAA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_sub_basic() {
        let instr = alu_imm(AL, 0x2, 0, 1, 0, 0, 10);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 90);
    }

    #[test]
    fn arm_subs_sets_carry_no_borrow() {
        let instr = alu_imm(AL, 0x2, 1, 1, 0, 0, 50);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 50);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 0);
    }

    #[test]
    fn arm_subs_borrow() {
        let instr = alu_imm(AL, 0x2, 1, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn arm_subs_overflow() {
        let instr = alu_imm(AL, 0x2, 1, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x8000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7FFF_FFFF);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 1);
    }

    #[test]
    fn arm_rsb_basic() {
        let instr = alu_imm(AL, 0x3, 0, 1, 0, 0, 100);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 30)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 70);
    }

    #[test]
    fn arm_rsbs_flags() {
        let instr = alu_imm(AL, 0x3, 1, 1, 0, 0, 10);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn arm_add_basic() {
        let instr = alu_imm(AL, 0x4, 0, 1, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 142);
    }

    #[test]
    fn arm_adds_carry_flag() {
        let instr = alu_imm(AL, 0x4, 1, 1, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn arm_adds_overflow_flag() {
        let instr = alu_imm(AL, 0x4, 1, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x7FFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x8000_0000);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 1);
    }

    #[test]
    fn arm_adds_no_carry_no_overflow() {
        let instr = alu_imm(AL, 0x4, 1, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 15);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 0);
    }

    #[test]
    fn arm_adc_with_carry() {
        let instr = alu_imm(AL, 0x5, 0, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 16);
    }

    #[test]
    fn arm_adc_without_carry() {
        let instr = alu_imm(AL, 0x5, 0, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 15);
    }

    #[test]
    fn arm_adcs_carry_out() {
        let instr = alu_imm(AL, 0x5, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_sbc_with_carry() {
        let instr = alu_imm(AL, 0x6, 0, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 20)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 15);
    }

    #[test]
    fn arm_sbc_without_carry() {
        let instr = alu_imm(AL, 0x6, 0, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 20)]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 14);
    }

    #[test]
    fn arm_rsc_with_carry() {
        let instr = alu_imm(AL, 0x7, 0, 1, 0, 0, 100);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 30)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 70);
    }

    #[test]
    fn arm_rsc_without_carry() {
        let instr = alu_imm(AL, 0x7, 0, 1, 0, 0, 100);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 30)]);
        cpu.set_cpsr_bit(C_FLAG, 0);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 69);
    }

    #[test]
    fn arm_tst_zero() {
        let instr = alu_imm(AL, 0x8, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xDEAD)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_tst_nonzero() {
        let instr = alu_imm(AL, 0x8, 1, 1, 0, 0, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn arm_teq_equal() {
        let instr = alu_imm(AL, 0x9, 1, 1, 0, 0, 0xAB);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xAB)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_teq_not_equal() {
        let instr = alu_imm(AL, 0x9, 1, 1, 0, 0, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    #[test]
    fn arm_cmp_equal() {
        let instr = alu_imm(AL, 0xA, 1, 1, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 42)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn arm_cmp_greater() {
        let instr = alu_imm(AL, 0xA, 1, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 0);
    }

    #[test]
    fn arm_cmp_less() {
        let instr = alu_imm(AL, 0xA, 1, 1, 0, 0, 10);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 5)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn arm_cmn_zero_result() {
        let instr = alu_imm(AL, 0xB, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_cmn_carry() {
        let instr = alu_imm(AL, 0xB, 1, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_orr_basic() {
        let instr = alu_imm(AL, 0xC, 0, 1, 0, 0, 0x0F);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xF0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFF);
    }

    #[test]
    fn arm_orrs_flags() {
        let instr = alu_imm(AL, 0xC, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_mov_imm() {
        let instr = alu_imm(AL, 0xD, 0, 0, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
    }

    #[test]
    fn arm_mov_with_rotation() {
        let instr = alu_imm(AL, 0xD, 0, 0, 0, 4, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFF00_0000);
    }

    #[test]
    fn arm_mov_reg() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xBEEF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xBEEF);
    }

    #[test]
    fn arm_movs_zero() {
        let instr = alu_imm(AL, 0xD, 1, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_bic_basic() {
        let instr = alu_imm(AL, 0xE, 0, 1, 0, 0, 0xFF);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xDEAD_BEEF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xDEAD_BEEF & !0xFF);
    }

    #[test]
    fn arm_mvn_basic() {
        let instr = alu_imm(AL, 0xF, 0, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
    }

    #[test]
    fn arm_mvns_flags() {
        let instr = alu_imm(AL, 0xF, 1, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 0);
    }

    // =======================================================================
    //  ALU SHIFTS (Immediate shifts)
    // =======================================================================

    #[test]
    fn arm_mov_lsl() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 4, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF0);
    }

    #[test]
    fn arm_mov_lsr() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 4, 1, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xF0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x0F);
    }

    #[test]
    fn arm_mov_asr_positive() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 4, 2, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x00F0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x000F);
    }

    #[test]
    fn arm_mov_asr_negative() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 4, 2, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xF000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFF00_0000);
    }

    #[test]
    fn arm_mov_ror() {
        let instr = alu_reg(AL, 0xD, 0, 0, 0, 4, 3, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0000_000F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF000_0000);
    }

    #[test]
    fn arm_mov_rrx() {
        let instr = alu_reg(AL, 0xD, 1, 0, 0, 0, 3, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x01)]);
        cpu.set_cpsr_bit(C_FLAG, 1);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x8000_0000);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
    }

    #[test]
    fn arm_lsl_shift_carry() {
        let instr = alu_reg(AL, 0xD, 1, 0, 0, 1, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x8000_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    // =======================================================================
    //  ALU SHIFTS (Register shifts: i=0, r=1)
    // =======================================================================

    #[test]
    fn arm_mov_lsl_by_reg() {
        let instr = alu_reg_shift_by_reg(AL, 0xD, 0, 0, 0, 2, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 5), (2, 3)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 40);
    }

    #[test]
    fn arm_mov_lsr_by_reg() {
        let instr = alu_reg_shift_by_reg(AL, 0xD, 0, 0, 0, 2, 1, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 64), (2, 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 16);
    }

    #[test]
    fn arm_mov_asr_by_reg() {
        let instr = alu_reg_shift_by_reg(AL, 0xD, 0, 0, 0, 2, 2, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x8000_0000), (2, 4)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xF800_0000);
    }

    #[test]
    fn arm_mov_ror_by_reg() {
        let instr = alu_reg_shift_by_reg(AL, 0xD, 0, 0, 0, 2, 3, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x1234_5678), (2, 8)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7812_3456);
    }

    // =======================================================================
    //  ALU with PC as destination (pipeline refill)
    // =======================================================================

    #[test]
    fn arm_mov_pc_refill() {
        let dest = IWRAM + 0x40;
        let instr = alu_reg(AL, 0xD, 0, 0, 15, 0, 0, 14);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(14, dest)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), dest & !3);
    }

    #[test]
    fn arm_movs_pc_restores_cpsr() {
        let dest = IWRAM + 0x60;
        let instr = alu_reg(AL, 0xD, 1, 0, 15, 0, 0, 14);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(14, dest)]);
        cpu.cpsr = 0x0000_00D3; // Supervisor mode
        cpu.spsr.insert(CPUMode::Supervisor, 0x6000_0010);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.cpsr, 0x6000_0010);
        assert_eq!(cpu.get_register_value(PC), dest & !3);
    }

    // =======================================================================
    //  MUL / MLA / UMULL / UMLAL / SMULL / SMLAL
    // =======================================================================

    #[test]
    fn arm_mul_basic() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 6), (3, 7)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 42);
    }

    #[test]
    fn arm_muls_zero() {
        let instr = mul_op(AL, 0b0000, 1, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 0), (3, 100)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }

    #[test]
    fn arm_muls_negative() {
        let instr = mul_op(AL, 0b0000, 1, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 0xFFFF_FFFF), (3, 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFE);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
    }

    #[test]
    fn arm_mla_basic() {
        let instr = mul_op(AL, 0b0001, 0, 0, 4, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 6), (3, 7), (4, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 52);
    }

    #[test]
    fn arm_umull_basic() {
        let instr = mul_op(AL, 0b0100, 0, 0, 1, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 0x1_0000), (3, 0x1_0000)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 1); // hi
        assert_eq!(cpu.get_register_value(1), 0); // lo
    }

    #[test]
    fn arm_smull_basic() {
        let instr = mul_op(AL, 0b0110, 0, 0, 1, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, (-3i32 as u32)), (3, 5)]);
        step(&mut cpu, &mut mem);
        let expected: u64 = (-15i64) as u64;
        assert_eq!(cpu.get_register_value(0), (expected >> 32) as u32);
        assert_eq!(cpu.get_register_value(1), expected as u32);
    }

    #[test]
    fn arm_umlal_basic() {
        let instr = mul_op(AL, 0b0101, 0, 0, 1, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0), (1, 10), (2, 5), (3, 3)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_register_value(1), 25);
    }

    #[test]
    fn arm_smlal_basic() {
        let instr = mul_op(
            AL,
            0b0111,
            0,
            0,
            1,
            3,
            2,
        );
        let (mut cpu, mut mem) = build(
            IWRAM,
            &[instr],
            &[(0, 0), (1, 100), (2, (-2i32 as u32)), (3, 10)],
        );
        step(&mut cpu, &mut mem);
        let expected: u64 = 80;
        assert_eq!(cpu.get_register_value(0), (expected >> 32) as u32);
        assert_eq!(cpu.get_register_value(1), expected as u32);
    }

    // =======================================================================
    //  SDT — LDR / STR (Immediate & Shifted Register Offsets)
    // =======================================================================

    #[test]
    fn arm_ldr_pre_index() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 1, 1, 0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA + 4, 0xDEAD_BEEF);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xDEAD_BEEF);
    }

    #[test]
    fn arm_ldr_post_index() {
        let instr = sdt_imm(AL, 0, 1, 0, 0, 1, 1, 0, 8);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA, 0xCAFE_BABE);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xCAFE_BABE);
        assert_eq!(cpu.get_register_value(1), DATA + 8);
    }

    #[test]
    fn arm_ldr_pre_writeback() {
        let instr = sdt_imm(AL, 1, 1, 0, 1, 1, 1, 0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA + 4, 0x1234_5678);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x1234_5678);
        assert_eq!(cpu.get_register_value(1), DATA + 4);
    }

    #[test]
    fn arm_ldrb() {
        let instr = sdt_imm(AL, 1, 1, 1, 0, 1, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write8(DATA, 0xAB);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xAB);
    }

    #[test]
    fn arm_str_pre_index() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 0, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xBEEF_CAFE), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA), 0xBEEF_CAFE);
    }

    #[test]
    fn arm_str_post_index() {
        let instr = sdt_imm(AL, 0, 1, 0, 0, 0, 1, 0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xAAAA_BBBB), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA), 0xAAAA_BBBB);
        assert_eq!(cpu.get_register_value(1), DATA + 4);
    }

    #[test]
    fn arm_strb() {
        let instr = sdt_imm(AL, 1, 1, 1, 0, 0, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xCD), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read8(DATA).0, 0xCD);
    }

    #[test]
    fn arm_ldr_down_offset() {
        let instr = sdt_imm(AL, 1, 0, 0, 0, 1, 1, 0, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA + 4)]);
        mem.write32(DATA, 0x1111_2222);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x1111_2222);
    }

    #[test]
    fn arm_ldr_reg_offset_lsl() {
        let instr = sdt_reg(AL, 1, 1, 0, 0, 1, 1, 0, 2, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 2)]);
        mem.write32(DATA + 8, 0x7777_8888);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7777_8888);
    }

    #[test]
    fn arm_str_reg_offset_lsl() {
        let instr = sdt_reg(AL, 1, 1, 0, 0, 0, 1, 0, 2, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x9999_0000), (1, DATA), (2, 3)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA + 12), 0x9999_0000);
    }

    #[test]
    fn arm_ldr_pc_destination() {
        let dest = IWRAM + 0x80;
        let instr = sdt_imm(AL, 1, 1, 0, 0, 1, 1, 15, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA, dest);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(PC), dest & !3);
    }

    // =======================================================================
    //  HSDT — LDRH, STRH, LDRSB, LDRSH (Immediate & Register Offsets)
    // =======================================================================

    #[test]
    fn arm_ldrh() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b01, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write16(DATA, 0xBEEF);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xBEEF);
    }

    #[test]
    fn arm_strh() {
        let instr = hsdt_imm(AL, 1, 1, 0, 0, 1, 0, 0b01, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xCAFE), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read16(DATA).0, 0xCAFE);
    }

    #[test]
    fn arm_ldrsb() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b10, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write8(DATA, 0x80);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FF80);
    }

    #[test]
    fn arm_ldrsh() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b11, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write16(DATA, 0x8000);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_8000);
    }

    #[test]
    fn arm_ldrsh_positive() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b11, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write16(DATA, 0x7FFF);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7FFF);
    }

    #[test]
    fn arm_ldrsb_positive() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b10, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write8(DATA, 0x7F);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x7F);
    }

    #[test]
    fn arm_ldrh_with_offset() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b01, 4);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write16(DATA + 4, 0x1234);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x1234);
    }

    #[test]
    fn arm_ldrh_reg_offset() {
        let instr = hsdt_reg(AL, 1, 1, 0, 1, 1, 0, 0b01, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 6)]);
        mem.write16(DATA + 6, 0x5678);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x5678);
    }

    #[test]
    fn arm_strh_reg_offset() {
        let instr = hsdt_reg(AL, 1, 1, 0, 0, 1, 0, 0b01, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x9ABC), (1, DATA), (2, 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.read16(DATA + 2).0, 0x9ABC);
    }

    // =======================================================================
    //  BDT — LDM / STM (IA, IB, DA, DB)
    // =======================================================================

    #[test]
    fn arm_stmia() {
        let rlist = (1 << 1) | (1 << 2) | (1 << 3);
        let instr = bdt(AL, 0, 1, 0, 1, 0, 0, rlist);
        let (mut cpu, mut mem) = build(
            IWRAM,
            &[instr],
            &[(0, DATA), (1, 0x11), (2, 0x22), (3, 0x33)],
        );
        step(&mut cpu, &mut mem);

        assert_eq!(mem.peek32(DATA), 0x11);
        assert_eq!(mem.peek32(DATA + 4), 0x22);
        assert_eq!(mem.peek32(DATA + 8), 0x33);
        assert_eq!(cpu.get_register_value(0), DATA + 12);
    }

    #[test]
    fn arm_ldmia() {
        let rlist = (1 << 1) | (1 << 2) | (1 << 3);
        let instr = bdt(AL, 0, 1, 0, 1, 1, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA)]);
        mem.write32(DATA, 0xAA);
        mem.write32(DATA + 4, 0xBB);
        mem.write32(DATA + 8, 0xCC);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(1), 0xAA);
        assert_eq!(cpu.get_register_value(2), 0xBB);
        assert_eq!(cpu.get_register_value(3), 0xCC);
        assert_eq!(cpu.get_register_value(0), DATA + 12);
    }

    #[test]
    fn arm_stmdb() {
        let rlist = (1 << 1) | (1 << 2);
        let instr = bdt(AL, 1, 0, 0, 1, 0, 0, rlist);
        let (mut cpu, mut mem) =
            build(IWRAM, &[instr], &[(0, DATA + 8), (1, 0x11), (2, 0x22)]);
        step(&mut cpu, &mut mem);

        assert_eq!(mem.peek32(DATA), 0x11);
        assert_eq!(mem.peek32(DATA + 4), 0x22);
        assert_eq!(cpu.get_register_value(0), DATA);
    }

    #[test]
    fn arm_ldmdb() {
        let rlist = (1 << 1) | (1 << 2);
        let instr = bdt(AL, 1, 0, 0, 1, 1, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA + 8)]);
        mem.write32(DATA, 0xDD);
        mem.write32(DATA + 4, 0xEE);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(1), 0xDD);
        assert_eq!(cpu.get_register_value(2), 0xEE);
        assert_eq!(cpu.get_register_value(0), DATA);
    }

    #[test]
    fn arm_stmib() {
        let rlist = 1 << 1;
        let instr = bdt(AL, 1, 1, 0, 1, 0, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA), (1, 0xFF)]);
        step(&mut cpu, &mut mem);

        assert_eq!(mem.peek32(DATA + 4), 0xFF);
        assert_eq!(cpu.get_register_value(0), DATA + 4);
    }

    #[test]
    fn arm_ldmda() {
        let rlist = (1 << 1) | (1 << 2);
        let instr = bdt(AL, 0, 0, 0, 1, 1, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA + 4)]);
        mem.write32(DATA, 0x11);
        mem.write32(DATA + 4, 0x22);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(1), 0x11);
        assert_eq!(cpu.get_register_value(2), 0x22);
        assert_eq!(cpu.get_register_value(0), DATA - 4);
    }

    #[test]
    fn arm_ldm_empty_rlist() {
        let instr = bdt(AL, 0, 1, 0, 1, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA)]);
        mem.write32(DATA, IWRAM + 0x20);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(0), DATA + 0x40);
    }

    #[test]
    fn arm_ldm_s_bit_restores_psr() {
        let rlist = (1 << 1) | (1 << 15);
        let instr = bdt(AL, 0, 1, 1, 0, 1, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA)]);
        cpu.cpsr = 0x0000_00D3; // Supervisor mode
        cpu.spsr.insert(CPUMode::Supervisor, 0x2000_0010);
        mem.write32(DATA, 0x1234);
        mem.write32(DATA + 4, IWRAM + 0x40);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(1), 0x1234);
        assert_eq!(cpu.cpsr, 0x2000_0010);
    }

    // =======================================================================
    //  SWP / SWPB
    // =======================================================================

    #[test]
    fn arm_swp() {
        let instr = swp(AL, 0, 1, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0x9999)]);
        mem.write32(DATA, 0xAAAA);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(0), 0xAAAA);
        assert_eq!(mem.peek32(DATA), 0x9999);
    }

    #[test]
    fn arm_swpb() {
        let instr = swp(AL, 1, 1, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0xEF)]);
        mem.write8(DATA, 0xAB);
        step(&mut cpu, &mut mem);

        assert_eq!(cpu.get_register_value(0), 0xAB);
        assert_eq!(mem.read8(DATA).0, 0xEF);
    }

    // =======================================================================
    //  MRS / MSR
    // =======================================================================

    #[test]
    fn arm_mrs_cpsr() {
        let instr = mrs(AL, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.cpsr = 0x6000_00D3;
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x6000_00D3);
    }

    #[test]
    fn arm_mrs_spsr() {
        let instr = mrs(AL, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.cpsr = 0x0000_00D3; // Supervisor mode
        cpu.spsr.insert(CPUMode::Supervisor, 0x9000_00D2);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x9000_00D2);
    }

    #[test]
    fn arm_msr_cpsr_flags_only() {
        let instr = msr_reg(AL, 0, 1, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xF000_0000)]);
        let old_cpsr = cpu.cpsr;
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.cpsr & 0xFF00_0000, 0xF000_0000);
        assert_eq!(cpu.cpsr & 0x00FF_FFFF, old_cpsr & 0x00FF_FFFF);
    }

    #[test]
    fn arm_msr_cpsr_control() {
        let instr = msr_reg(AL, 0, 0, 0, 0, 1, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x0000_001F)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.cpsr & 0xDF, 0x1F);
    }

    #[test]
    fn arm_msr_imm_flags() {
        let instr = msr_imm(AL, 0, 1, 0, 0, 0, 2, 0x0F);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.cpsr & 0xF000_0000, 0xF000_0000);
    }

    #[test]
    fn arm_msr_spsr_reg() {
        let instr = msr_reg(AL, 1, 1, 0, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xC000_0000)]);
        cpu.cpsr = 0x0000_00D3; // Supervisor
        cpu.spsr.insert(CPUMode::Supervisor, 0x0000_0000);
        step(&mut cpu, &mut mem);
        let spsr = *cpu.spsr.get(&CPUMode::Supervisor).unwrap();
        assert_eq!(spsr & 0xFF00_0000, 0xC000_0000);
    }

    // =======================================================================
    //  ALU Immediate with Rotation
    // =======================================================================

    #[test]
    fn arm_add_rotated_imm() {
        let instr = alu_imm(AL, 0x4, 0, 1, 0, 8, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x0001_0000);
    }

    // =======================================================================
    //  CLK TIMING TESTS (ARM)
    // =======================================================================

    #[test]
    fn arm_alu_imm_clk() {
        let instr = alu_imm(AL, 0x4, 0, 1, 0, 0, 5);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 10)]);
        assert_eq!(step(&mut cpu, &mut mem), 1);
    }

    #[test]
    fn arm_alu_reg_shift_by_reg_clk() {
        let instr = alu_reg_shift_by_reg(AL, 0xD, 0, 0, 0, 2, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFF), (2, 4)]);
        assert_eq!(step(&mut cpu, &mut mem), 2);
    }

    #[test]
    fn arm_mul_clk_rs_count_ge_24() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x55)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 1S + 1
    }

    #[test]
    fn arm_mul_clk_rs_count_16_to_24() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x5500)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 2
    }

    #[test]
    fn arm_mul_clk_rs_count_8_to_16() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x0055_0000)]);
        assert_eq!(step(&mut cpu, &mut mem), 4); // 1S + 3
    }

    #[test]
    fn arm_mul_clk_rs_count_under_8() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x5500_0000)]);
        assert_eq!(step(&mut cpu, &mut mem), 5); // 1S + 4
    }

    #[test]
    fn arm_mla_clk() {
        let instr = mul_op(AL, 0b0001, 0, 0, 4, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x55), (4, 0)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1(m) + 1
    }

    #[test]
    fn arm_ldr_iwram_clk() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 1, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1N + 1I
    }

    #[test]
    fn arm_str_iwram_clk() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 0, 1, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0), (1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 1S + 1N
    }

    #[test]
    fn arm_hsdt_strh_clk() {
        let instr = hsdt_imm(AL, 1, 1, 0, 0, 1, 0, 0b01, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0x1234), (1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 2); // 1S + 1N
    }

    #[test]
    fn arm_hsdt_ldrh_clk() {
        let instr = hsdt_imm(AL, 1, 1, 0, 1, 1, 0, 0b01, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1N + 1I
    }

    #[test]
    fn arm_swp_iwram_clk() {
        let instr = swp(AL, 0, 1, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA), (2, 0)]);
        assert_eq!(step(&mut cpu, &mut mem), 4); // 1S + 1N + 1N + 1I
    }

    #[test]
    fn arm_b_iwram_clk() {
        let instr = branch(AL, 0, 0x00FF_FFFE);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + refill(1N + 1S)
    }

    #[test]
    fn arm_bx_iwram_clk() {
        let instr = bx(AL, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, IWRAM + 0x40)]);
        assert_eq!(step(&mut cpu, &mut mem), 3);
    }

    #[test]
    fn arm_ldm_3_regs_clk() {
        let rlist = (1 << 1) | (1 << 2) | (1 << 3);
        let instr = bdt(AL, 0, 1, 0, 0, 1, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA)]);
        assert_eq!(step(&mut cpu, &mut mem), 5); // 1S + 3 reads + 1I
    }

    #[test]
    fn arm_stm_2_regs_clk() {
        let rlist = (1 << 1) | (1 << 2);
        let instr = bdt(AL, 0, 1, 0, 1, 0, 0, rlist);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, DATA), (1, 0), (2, 0)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 2 writes
    }

    #[test]
    fn arm_umull_clk() {
        let instr = mul_op(AL, 0b0100, 0, 0, 1, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 5), (3, 0x55)]);
        assert_eq!(step(&mut cpu, &mut mem), 3); // 1S + 1(m) + 1
    }

    #[test]
    fn arm_umlal_clk() {
        let instr = mul_op(AL, 0b0101, 0, 0, 1, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0), (1, 0), (2, 5), (3, 0x55)]);
        assert_eq!(step(&mut cpu, &mut mem), 4); // 1S + 1(m) + 2
    }

    #[test]
    fn arm_failed_cond_clk() {
        let instr = alu_imm(0x0, 0xD, 0, 0, 0, 0, 42);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 0);
        assert_eq!(step(&mut cpu, &mut mem), 1);
    }

    // =======================================================================
    //  Edge Cases & Flags
    // =======================================================================

    #[test]
    fn arm_add_wraps() {
        let instr = alu_imm(AL, 0x4, 0, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    #[test]
    fn arm_sub_wraps() {
        let instr = alu_imm(AL, 0x2, 0, 1, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
    }

    #[test]
    fn arm_mul_wrapping() {
        let instr = mul_op(AL, 0b0000, 0, 0, 0, 3, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(2, 0x8000_0000), (3, 2)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    #[test]
    fn arm_and_all_ones() {
        let instr = alu_reg(AL, 0x0, 0, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF), (2, 0xFFFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
    }

    #[test]
    fn arm_orr_complementary() {
        let instr = alu_reg(AL, 0xC, 0, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x0F0F_0F0F), (2, 0xF0F0_F0F0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFF);
    }

    #[test]
    fn arm_bic_clear_bits() {
        let instr = alu_reg(AL, 0xE, 0, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xFFFF_FFFF), (2, 0x0000_00FF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FF00);
    }

    #[test]
    fn arm_eor_self_is_zero() {
        let instr = alu_reg(AL, 0x1, 0, 1, 0, 0, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0xDEAD_BEEF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
    }

    #[test]
    fn arm_mvn_double_is_identity() {
        let instr1 = alu_reg(AL, 0xF, 0, 0, 0, 0, 0, 1);
        let instr2 = alu_reg(AL, 0xF, 0, 0, 2, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr1, instr2], &[(1, 0xCAFE)]);
        step(&mut cpu, &mut mem);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(2), 0xCAFE);
    }

    #[test]
    fn arm_subs_zero_minus_zero() {
        let instr = alu_imm(AL, 0x2, 1, 1, 0, 0, 0);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 0);
    }

    #[test]
    fn arm_adds_max_plus_max() {
        let instr = alu_reg(AL, 0x4, 1, 1, 0, 0, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 0x7FFF_FFFF), (2, 0x7FFF_FFFF)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0xFFFF_FFFE);
        assert_eq!(cpu.get_cpsr_bit(V_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(N_FLAG), 1);
        assert_eq!(cpu.get_cpsr_bit(C_FLAG), 0);
    }

    #[test]
    fn arm_ldr_unaligned_rotates() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 1, 1, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, DATA)]);
        mem.write32(DATA, 0x04030201);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 0x02010403);
    }

    #[test]
    fn arm_str_unaligned_force_aligns() {
        let instr = sdt_imm(AL, 1, 1, 0, 0, 0, 1, 0, 1);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(0, 0xDEAD_BEEF), (1, DATA)]);
        step(&mut cpu, &mut mem);
        assert_eq!(mem.peek32(DATA), 0xDEAD_BEEF);
    }

    #[test]
    fn arm_add_with_lsl_shift() {
        let instr = alu_reg(AL, 0x4, 0, 1, 0, 2, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100), (2, 5)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 120);
    }

    #[test]
    fn arm_sub_with_lsr_shift() {
        let instr = alu_reg(AL, 0x2, 0, 1, 0, 1, 1, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 100), (2, 20)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_register_value(0), 90);
    }

    #[test]
    fn arm_cmp_with_shifted_operand() {
        let instr = alu_reg(AL, 0xA, 1, 1, 0, 1, 0, 2);
        let (mut cpu, mut mem) = build(IWRAM, &[instr], &[(1, 20), (2, 10)]);
        step(&mut cpu, &mut mem);
        assert_eq!(cpu.get_cpsr_bit(Z_FLAG), 1);
    }
}
