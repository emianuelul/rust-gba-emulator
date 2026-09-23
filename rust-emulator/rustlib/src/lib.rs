use crate::{cpu_module::CPU, gba_emulator::GBAEngine, memory_area::GBAMemory};
use std::fs;

pub mod cpu_module;
pub mod gba_emulator;
pub mod memory_area;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_tests() {
        let rom = fs::read("/Users/iemi/Downloads/gba-tests/FuzzARM/ARM_DataProcessing.gba")
            .expect("ARM test not found");

        let mut mem = GBAMemory::new(rom);
        let mut cpu = CPU::new();

        let mut prev_pc = u32::MAX;
        let mut stable_count = 0;
        const STABLE_THRESHOLD: u32 = 50;

        let mut counter: usize = 0;
        loop {
            let _ = cpu.step(&mut mem);
            counter += 1;

            let pc = cpu.get_register_value(15);

            if pc == prev_pc {
                stable_count += 1;
                if stable_count >= STABLE_THRESHOLD {
                    break;
                }
            } else {
                stable_count = 0;
                prev_pc = pc;
            }
        }

        let (marker, _) = mem.read32(0x0200_0000);
        println!(
            "Finished after {} steps, landed at PC = {:x}",
            counter,
            cpu.get_register_value(15)
        );
        assert_eq!(marker, 0, "a test failed, eWRAM marker = {:#010x}", marker);
    }
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
