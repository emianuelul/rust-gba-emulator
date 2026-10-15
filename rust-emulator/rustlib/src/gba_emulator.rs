use crate::cpu::CPU;
use crate::memory_area::GBAMemory;
use crate::ppu_module::PPU;

pub struct GBAEngine {
    memory: GBAMemory,
    cpu: CPU,
    ppu: PPU,
}

impl GBAEngine {
    pub fn new(rom_data: Vec<u8>) -> Self {
        GBAEngine {
            memory: GBAMemory::new(rom_data),
            cpu: CPU::new(),
            ppu: PPU::new(),
        }
    }
}

impl GBAEngine {
    pub fn step(&mut self) {
        let clk = self.cpu.step(&mut self.memory);
        self.ppu.step(&mut self.memory, clk);
    }
}
