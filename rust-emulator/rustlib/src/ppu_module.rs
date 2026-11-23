use crate::bgmodes_draw::{bgm3_draw_scanline, bgm4_draw_scanline, bgm5_draw_scanline};
use crate::constants::{
    HDRAW_CYCLES, HDRAW_PIXELS, PRAM_START, SCANLINE_CYCLES, VBLANK_LINES, VDRAW_LINES, VRAM_START,
};
use crate::display_registers::{
    RegDispcnt, RegDispstat, RegVcount, DISPCNT_ADDR, DISPSTAT_ADDR, VCOUNT_ADDR,
};

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
    fn draw_scanline(&mut self, memory: &mut GBAMemory, scanline_index: u8) {
        let dispcnt: RegDispcnt = memory.read16(DISPCNT_ADDR).0.into();

        match dispcnt.bg_mode {
            0..=2 => {
                // sprite modes
                match dispcnt.bg_mode {
                    0 => {
                        println!("BGM 0 not implemented")
                    }

                    1 => {
                        println!("BGM 1 not implemented")
                    }

                    2 => {
                        println!("BGM 2 not implemented")
                    }

                    _ => {
                        unreachable!()
                    }
                }
            }

            3..=5 => {
                // bitmap modes

                let selected_frame_addr = if dispcnt.frame_select as u8 == 0 {
                    // frame 0
                    VRAM_START
                } else {
                    // frame 1
                    VRAM_START + 0xA000
                };

                let scanline = match dispcnt.bg_mode {
                    3 => bgm3_draw_scanline(memory, scanline_index),

                    4 => bgm4_draw_scanline(memory, scanline_index, selected_frame_addr),

                    5 => bgm5_draw_scanline(memory, scanline_index, selected_frame_addr),

                    _ => {
                        unreachable!()
                    }
                };

                for i in 0..(HDRAW_PIXELS as u32) {
                    let arr_index = (scanline_index as u32 * HDRAW_PIXELS as u32 + i) as usize;

                    self.display.screen[arr_index] = scanline[i as usize];
                }
            }

            _ => {
                tracing::error!("Invalid background mode: {}", dispcnt.bg_mode);
            }
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

        let mut dispstat: RegDispstat = memory.read16(DISPSTAT_ADDR).0.into();
        let mut vcount: RegVcount = memory.read16(VCOUNT_ADDR).0.into();

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
