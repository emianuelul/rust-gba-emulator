use crate::display_registers::*;
use crate::memory_area::GBAMemory;

pub struct PPU {
    display: GBADisplay,
    cycles: usize,
}

pub struct GBADisplay {
    screen: [[u16; 240]; 160],
    rot_scal: bool,
    tilemap: bool,
}

impl GBADisplay {
    pub fn new() -> Self {
        GBADisplay {
            screen: [[0; 240]; 160],
            rot_scal: false,
            tilemap: false,
        }
    }
}

// init
impl PPU {
    pub fn new() -> Self {
        PPU {
            display: GBADisplay::new(),
            cycles: 0,
        }
    }
}

impl Default for PPU {
    fn default() -> PPU {
        PPU::new()
    }
}

// DISPCNT Handling
impl PPU {}

impl PPU {
    fn step(&mut self, clk: u32, memory: &mut GBAMemory) {
        self.cycles = self.cycles.wrapping_add(clk as usize);
    }
}
