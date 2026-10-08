use crate::constants::{HDRAW_CYCLES, SCANLINE_CYCLES, VBLANK_LINES, VDRAW_LINES};
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

impl Default for GBADisplay {
    fn default() -> GBADisplay {
        GBADisplay::new()
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
    pub fn step(&mut self, memory: &mut GBAMemory, clk: u32) {
        self.cycles += clk as usize;

        // Handle HDraw transition to HBlank (past 960 cycles)
        if self.cycles >= HDRAW_CYCLES as usize && dispstat_get_hblank(memory) == 0 {
            if get_vcount(memory) <= VDRAW_LINES as usize {
                // draw_scanline()
            }
            dispstat_set_hblank(memory, 1);
        }

        // Handle HBlank and advance VBlank
        if self.cycles >= SCANLINE_CYCLES as usize {
            self.cycles -= SCANLINE_CYCLES as usize;
            dispstat_set_hblank(memory, 0);

            let next_vcount = (get_vcount(memory) as u16 + 1) as u32;
            if next_vcount >= VBLANK_LINES {
                set_vcount(memory, 0);
                dispstat_set_vblank(memory, 0);
            } else {
                set_vcount(memory, next_vcount as u16);
                if next_vcount >= VDRAW_LINES {
                    dispstat_set_vblank(memory, 1);
                }
            }
        }
    }
}
