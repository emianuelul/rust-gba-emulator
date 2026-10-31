use crate::constants::{
    HDRAW_CYCLES, HDRAW_PIXELS, SCANLINE_CYCLES, VBLANK_LINES, VDRAW_LINES, VRAM_START,
};
use crate::display_registers::{DISPSTAT, DISPSTAT_ADDR, VCOUNT, VCOUNT_ADDR};
use crate::memory_area::GBAMemory;

pub struct PPU {
    pub display: GBADisplay,
    cycles: usize,
}

pub struct GBADisplay {
    pub screen: [u32; 240 * 160],
}

impl GBADisplay {
    pub fn new() -> Self {
        GBADisplay {
            screen: [0; 240 * 160],
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
    fn convert_bgr555_hex(&self, bgr: u16) -> u32 {
        let r = 0b11111 & bgr;
        let g = 0b11111 & (bgr >> 5);
        let b = 0b11111 & (bgr >> 10);

        let r8 = ((r << 3) | (r >> 2)) as u32;
        let g8 = ((g << 3) | (g >> 2)) as u32;
        let b8 = ((b << 3) | (b >> 2)) as u32;

        (0xFF << 24) | (b8 << 16) | (g8 << 8) | r8
    }

    fn draw_scanline(&mut self, memory: &mut GBAMemory, scanline_index: u8) {
        // MODE 3 TO START
        // MODE 3 - 15bit colors, no pallette ram

        for i in 0..(HDRAW_PIXELS as u32) {
            let arr_index = (scanline_index as u32 * HDRAW_PIXELS as u32 + i) as usize;
            let addr = VRAM_START + (arr_index as u32) * 2;
            let pixel = memory.read16(addr).0;

            self.display.screen[arr_index] = self.convert_bgr555_hex(pixel);
        }
    }

    pub fn get_frame(&self) -> [u32; 240 * 160] {
        self.display.screen
    }
}

impl PPU {
    pub fn step(&mut self, memory: &mut GBAMemory, clk: u32) -> bool {
        let mut frame_ready: bool = false;
        self.cycles += clk as usize;

        let mut dispstat: DISPSTAT = memory.read16(DISPSTAT_ADDR).0.into();
        let mut vcount: VCOUNT = memory.read16(VCOUNT_ADDR).0.into();

        // transition from HDRAW to HBLANK
        if self.cycles >= HDRAW_CYCLES as usize && !dispstat.hblank_flag {
            if vcount.current_scanline < VDRAW_LINES {
                self.draw_scanline(memory, vcount.current_scanline);
            }
            dispstat.hblank_flag = true;
        }

        // Handle HBlank and advance VBlank
        if self.cycles >= SCANLINE_CYCLES as usize {
            self.cycles -= SCANLINE_CYCLES as usize;

            dispstat.hblank_flag = false;

            let next = vcount.current_scanline + 1;

            // reset drawing frame if on last line
            if next >= VBLANK_LINES + VDRAW_LINES {
                vcount.current_scanline = 0;
                dispstat.vblank_flag = false;
            } else {
                vcount.current_scanline = next;
                if next == VDRAW_LINES {
                    dispstat.vblank_flag = true;
                    frame_ready = true;
                }
            }

            dispstat.vcounter_flag = vcount.current_scanline == dispstat.vcount_setting;
            memory.write16(VCOUNT_ADDR, vcount.into());
        }

        memory.write16(DISPSTAT_ADDR, dispstat.into());
        frame_ready
    }
}
