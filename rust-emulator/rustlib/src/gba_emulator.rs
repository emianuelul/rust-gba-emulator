use crate::cpu::Cpu;
use crate::memory_area::GBAMemory;
use crate::ppu_module::PPU;

pub struct GBAEngine {
    memory: GBAMemory,
    cpu: Cpu,
    ppu: PPU,
    pending_clk: u32,
}

impl GBAEngine {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let mut engine = GBAEngine {
            memory: GBAMemory::new(rom_data),
            cpu: Cpu::new(),
            ppu: PPU::new(),
            pending_clk: 0,
        };

        // init first 2 fetches
        engine.pending_clk = engine
            .cpu
            .get_refill_clk(&engine.memory, engine.cpu.registers.pc);

        engine
    }
}

pub struct Result {
    pub draw: bool,
    pub screen: [u32; 240 * 160],
}

impl GBAEngine {
    pub fn step(&mut self) -> Result {
        let mut screen: [u32; 240 * 160] = [0; 240 * 160];
        let mut draw = false;

        let clk = std::mem::take(&mut self.pending_clk) + self.cpu.step(&mut self.memory);
        if self.ppu.step(&mut self.memory, clk) {
            //println!("====== DRAWING FRAME ======");
            //println!("{:?}", self.ppu.get_frame());
            draw = true;
            screen = self.ppu.get_frame();
        }

        Result { draw, screen }
    }
}
