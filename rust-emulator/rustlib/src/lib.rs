pub mod constants;
pub mod cpu_arm_ops;
pub mod cpu_module;
pub mod cpu_thumb_ops;
pub mod gba_emulator;
pub mod memory_area;
pub mod ppu_module;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cpu_module::CPU, memory_area::GBAMemory};
    use std::collections::{HashSet, VecDeque};

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::TRACE)
            .try_init();
    }

    #[test]
    fn tests() {
        init_tracing();

        let tests = [
            "/Users/iemi/Downloads/gba-tests/FuzzARM/ARM_DataProcessing.gba",
            "/Users/iemi/Downloads/gba-tests/FuzzARM/ARM_Any.gba",
            "/Users/iemi/Downloads/gba-tests/FuzzARM/THUMB_Any.gba",
            "/Users/iemi/Downloads/gba-tests/FuzzARM/THUMB_DataProcessing.gba",
            "/Users/iemi/Downloads/gba-tests/gba-tests/thumb/thumb.gba",
            "/Users/iemi/Downloads/gba-tests/gba-tests/arm/arm.gba",
        ];

        let rom = std::fs::read(tests[5]).expect("ARM test not found");

        let mut mem = GBAMemory::new(rom);
        let mut cpu = CPU::new();

        let mut counter: u32 = 0;

        let cap = 128;
        let mut last_few_pc: VecDeque<u32> = VecDeque::with_capacity(cap);
        let threshold = 10;
        let mut step: usize = 0;
        let window_count = 2000;

        loop {
            let _ = cpu.step(&mut mem);
            counter = counter.wrapping_add(1);
            step = step.wrapping_add(1);

            let pc = cpu.get_register_value(15);
            last_few_pc.push_back(pc);

            if last_few_pc.len() == cap {
                if step >= window_count {
                    step = 0;
                    let mut freq: HashSet<u32> = HashSet::new();
                    for &i in last_few_pc.iter() {
                        freq.insert(i);
                    }

                    if freq.len() <= threshold {
                        println!("Detected loop, breaking...");
                        break;
                    }
                }
                last_few_pc.pop_front();
            }

            if pc == 0x8001d4c {
                println!("gba-tests ended | R12 = {}", cpu.get_register_value(12));
                break;
            }

            if counter == u32::MAX {
                println!("Ran too long, breaking...");
                break;
            }
        }

        println!(
            "Finished after {} steps, landed at PC = {:x}",
            counter,
            cpu.get_register_value(15)
        );
    }
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
