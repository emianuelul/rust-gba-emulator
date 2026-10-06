use crate::display_registers::*;
use crate::memory_area::GBAMemory;

pub struct PPU {
    display: GBADisplay,
}

// layers: 0123
// features: Scrolling|Flip|Mosaic|AlphaBlending|Brightness|Priority
pub struct GBADisplay {
    screen: [[u16; 240]; 160],
    green_swap: bool,
    rot_scal: bool,
    layers: (bool, bool, bool, bool),
    features: (bool, bool, bool, bool, bool, bool),
    tilemap: bool,
}

impl GBADisplay {
    pub fn new() -> Self {
        GBADisplay {
            screen: [[0; 240]; 160],
            green_swap: false,
            rot_scal: false,
            layers: (false, false, false, false),
            features: (false, false, false, false, false, false),
            tilemap: false,
        }
    }
}

// init
impl PPU {
    pub fn new() -> Self {
        PPU {
            display: GBADisplay::new(),
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
    fn step(&mut self, clk: u32, memory: &mut GBAMemory) {}
}
