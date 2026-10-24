#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::cpu::Cpu;
    use crate::memory_area::GBAMemory;

    const IWRAM: u32 = 0x0300_0000;
    const DATA: u32 = 0x0300_0100;
    const ROM: u32 = 0x0800_0000;

    const MOV_R0_0: u32 = 0x2000; // MOV r0,#0
    const B_SELF: u32 = 0xE7FE; // B .
    const BEQ_SELF: u32 = 0xD0FE; // BEQ .
    const BNE_SELF: u32 = 0xD1FE; // BNE .
    const LDR_R1_R2: u32 = 0x6811; // LDR r1,[r2]
    const STR_R1_R2: u32 = 0x6011; // STR r1,[r2]
    const PUSH_R0: u32 = 0xB401; // PUSH {r0}
    const POP_PC: u32 = 0xBD00; // POP {pc}
    const MUL_R0_R1: u32 = 0x4348; // MUL r0,r1
    const BX_R0: u32 = 0x4700; // BX r0
    const BL_HI: u32 = 0xF000; // BL, first half  (offset 0)
    const BL_LO: u32 = 0xF800; // BL, second half (offset 0)

    fn build(base: u32, program: &[u32], regs: &[(usize, u32)]) -> (Cpu, GBAMemory) {
        let (mut cpu, mem) = if base >= ROM {
            let mut rom = vec![0u8; 0x100];
            for (i, h) in program.iter().enumerate() {
                rom[i * 2..i * 2 + 2].copy_from_slice(&(*h as u16).to_le_bytes());
            }
            (Cpu::new(), GBAMemory::new(rom))
        } else {
            let mut mem = GBAMemory::new(vec![0u8; 0x100]);
            for (i, h) in program.iter().enumerate() {
                mem.write16(base + i as u32 * 2, *h as u16);
            }
            (Cpu::new(), mem)
        };

        cpu.set_cpsr_bit(T_FLAG, 1);
        for &(r, v) in regs {
            cpu.set_register_value(r, v);
        }
        cpu.registers.pc = base;
        let _ = cpu.get_refill_clk(&mem, base);
        cpu.registers.pc = base;
        (cpu, mem)
    }

    fn steps(cpu: &mut Cpu, mem: &mut GBAMemory, n: usize) -> Vec<u32> {
        (0..n).map(|_| cpu.step(mem)).collect()
    }

    #[test]
    fn thumb_alu_is_1s() {
        let (mut cpu, mut mem) = build(IWRAM, &[MOV_R0_0], &[]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![1]);
    }

    #[test]
    fn thumb_branch_is_2s_1n() {
        let (mut cpu, mut mem) = build(IWRAM, &[B_SELF], &[]);
        assert_eq!(steps(&mut cpu, &mut mem, 3), vec![3, 3, 3]);
    }

    #[test]
    fn thumb_cond_branch_taken_is_2s_1n() {
        let (mut cpu, mut mem) = build(IWRAM, &[BEQ_SELF], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        assert_eq!(steps(&mut cpu, &mut mem, 2), vec![3, 3]);
    }

    #[test]
    fn thumb_cond_branch_not_taken_is_1s() {
        let (mut cpu, mut mem) = build(IWRAM, &[BNE_SELF], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![1]);
    }

    #[test]
    fn thumb_bl_is_1s_then_2s_1n() {
        let (mut cpu, mut mem) = build(IWRAM, &[BL_HI, BL_LO], &[]);
        assert_eq!(steps(&mut cpu, &mut mem, 2), vec![1, 3]);
    }

    #[test]
    fn thumb_bx_is_2s_1n() {
        let (mut cpu, mut mem) = build(IWRAM, &[BX_R0], &[(0, IWRAM + 0x40)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![3]);
    }

    #[test]
    fn thumb_ldr_is_1s_1n_1i() {
        let (mut cpu, mut mem) = build(IWRAM, &[LDR_R1_R2], &[(2, DATA)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![3]);
    }

    #[test]
    fn thumb_str_is_2n() {
        let (mut cpu, mut mem) = build(IWRAM, &[STR_R1_R2], &[(2, DATA)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![2]);
    }

    #[test]
    fn thumb_push_one_reg_is_2n() {
        let (mut cpu, mut mem) = build(IWRAM, &[PUSH_R0], &[(SP, DATA + 0x80)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![2]);
    }

    #[test]
    fn thumb_pop_pc_is_2s_2n_1i() {
        let (mut cpu, mut mem) = build(IWRAM, &[POP_PC], &[(SP, DATA)]);
        mem.write32(DATA, (IWRAM + 0x40) | 1);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![5]);
    }

    #[test]
    fn thumb_mul_cost_depends_on_the_multiplier() {
        let (mut cpu, mut mem) = build(IWRAM, &[MUL_R0_R1], &[(0, 0xFF), (1, 3)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![2]);

        let (mut cpu, mut mem) = build(IWRAM, &[MUL_R0_R1], &[(0, 0x0100_0000), (1, 3)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![5]);
    }

    #[test]
    fn thumb_rom_taken_branch_exact_and_stable() {
        let (mut cpu, mut mem) = build(ROM, &[MOV_R0_0, B_SELF], &[]);
        let b = ROM + 2;

        let expected = mem.get_access_clk(b, 2, true)
            + mem.get_access_clk(b, 2, false)
            + mem.get_access_clk(b + 2, 2, true);

        let c = steps(&mut cpu, &mut mem, 3);

        assert_eq!(c[1], expected, "B, first time (fetch S + refill N + S)");
        assert_eq!(
            c[2], expected,
            "B again after a refill: its fetch must be S, not N"
        );
    }
}
