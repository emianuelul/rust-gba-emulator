#[cfg(test)]
mod tests {
    use crate::constants::*;
    use crate::cpu::Cpu;
    use crate::memory_area::GBAMemory;

    const IWRAM: u32 = 0x0300_0000;
    const DATA: u32 = 0x0300_0100;
    const ROM: u32 = 0x0800_0000;

    const MOV_R0_R0: u32 = 0xE1A0_0000;
    const B_SELF: u32 = 0xEAFF_FFFE; // B .
    const BNE_SELF: u32 = 0x1AFF_FFFE; // BNE .
    const LDR_R1_R2: u32 = 0xE592_1000; // LDR r1,[r2]
    const STR_R1_R2: u32 = 0xE582_1000; // STR r1,[r2]
    const LDM_R2_345: u32 = 0xE892_0038; // LDMIA r2,{r3-r5}

    fn build(base: u32, program: &[u32], regs: &[(usize, u32)]) -> (Cpu, GBAMemory) {
        let (mut cpu, mem) = if base >= ROM {
            let mut rom = vec![0u8; 0x100];
            for (i, w) in program.iter().enumerate() {
                rom[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
            }
            (Cpu::new(), GBAMemory::new(rom))
        } else {
            let mut mem = GBAMemory::new(vec![0u8; 0x100]);
            for (i, w) in program.iter().enumerate() {
                mem.write32(base + i as u32 * 4, *w);
            }
            (Cpu::new(), mem)
        };

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
    fn iwram_alu_is_1s() {
        let (mut cpu, mut mem) = build(IWRAM, &[MOV_R0_R0], &[]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![1]);
    }

    #[test]
    fn iwram_branch_is_2s_1n() {
        let (mut cpu, mut mem) = build(IWRAM, &[B_SELF], &[]);
        assert_eq!(steps(&mut cpu, &mut mem, 3), vec![3, 3, 3]);
    }

    #[test]
    fn iwram_ldr_is_1s_1n_1i() {
        let (mut cpu, mut mem) = build(IWRAM, &[LDR_R1_R2], &[(2, DATA)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![3]);
    }

    #[test]
    fn iwram_str_is_2n() {
        let (mut cpu, mut mem) = build(IWRAM, &[STR_R1_R2], &[(2, DATA)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![2]);
    }

    #[test]
    fn iwram_ldm_3_regs_is_3s_1n_1i() {
        let (mut cpu, mut mem) = build(IWRAM, &[LDM_R2_345], &[(2, DATA)]);
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![5]);
    }

    #[test]
    fn failed_condition_costs_only_the_fetch() {
        let (mut cpu, mut mem) = build(IWRAM, &[BNE_SELF], &[]);
        cpu.set_cpsr_bit(Z_FLAG, 1); // BNE not taken
        assert_eq!(steps(&mut cpu, &mut mem, 1), vec![1]);
    }

    #[test]
    fn rom_taken_branch_exact_and_stable() {
        let (mut cpu, mut mem) = build(ROM, &[MOV_R0_R0, B_SELF], &[]);
        let b = ROM + 4;

        let expected = mem.get_access_clk(b, 4, true)
            + mem.get_access_clk(b, 4, false)
            + mem.get_access_clk(b + 8, 4, true);

        let c = steps(&mut cpu, &mut mem, 3);

        assert_eq!(c[1], expected, "B, first time (fetch S + refill N + S)");
        assert_eq!(
            c[2], expected,
            "B again after a refill: its fetch must be S, not N"
        );
    }
}
