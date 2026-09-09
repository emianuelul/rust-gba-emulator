use crate::cpu_module::*;
use crate::memory_area::*;

pub struct GBAEngine {
    memory: GBAMemory,
    cpu: CPU,
}

impl GBAEngine {
    pub fn new(rom_data: Vec<u8>) -> Self {
        GBAEngine {
            memory: GBAMemory::new(rom_data),
            cpu: CPU::new(),
        }
    }
}
