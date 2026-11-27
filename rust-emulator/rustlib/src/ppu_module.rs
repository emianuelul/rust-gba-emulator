use crate::bgmodes_draw::{
    bgm0_draw_scanline, bgm1_draw_scanline, bgm3_draw_scanline, bgm4_draw_scanline,
    bgm5_draw_scanline, BGLayer,
};

use crate::constants::{
    HDRAW_CYCLES, HDRAW_PIXELS, KB, SCANLINE_CYCLES, VBLANK_LINES, VDRAW_LINES, VRAM_START,
};

use crate::display_registers::{
    get_bgxcnt, get_bgxhofs, get_bgxvofs, RegDispcnt, RegDispstat, RegVcount, DISPCNT_ADDR,
    DISPSTAT_ADDR, VCOUNT_ADDR,
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

fn init_active_layers(dispcnt: RegDispcnt) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    if dispcnt.disp_bg0 {
        result.push(0);
    }
    if dispcnt.disp_bg1 {
        result.push(1);
    }
    if dispcnt.disp_bg2 {
        result.push(2);
    }
    if dispcnt.disp_bg3 {
        result.push(3);
    }

    result
}

fn init_layers(memory: &GBAMemory, active_layers: Vec<u8>, bg_mode: u8) -> Vec<BGLayer> {
    let mut result: Vec<BGLayer> = Vec::new();

    for active_layer in active_layers {
        let cnt = get_bgxcnt(memory, active_layer);
        let xy = (
            get_bgxhofs(memory, active_layer).offset,
            get_bgxvofs(memory, active_layer).offset,
        );
        let sc_addr = VRAM_START + cnt.screen_base_block as u32 * 2 * KB;
        let (width, height) = match cnt.screen_size {
            0 => (256, 256),
            1 => (512, 256),
            2 => (256, 512),
            3 => (512, 512),
            _ => {
                unreachable!()
            }
        };

        let kind = matches!((bg_mode, active_layer), (1, 2) | (2, 2) | (2, 3));

        result.push(BGLayer {
            bg_index: active_layer as usize,
            cnt,
            xy,
            sc_addr,
            width,
            height,
            scanline: Vec::new(),
            kind,
        });
    }

    result
}

impl PPU {
    fn draw_scanline(&mut self, memory: &mut GBAMemory, screen_y: u8) {
        let dispcnt: RegDispcnt = memory.read16(DISPCNT_ADDR).0.into();
        let scanline: Vec<u32> = match dispcnt.bg_mode {
            // tile modes
            0..=2 => {
                let active_layers = init_active_layers(dispcnt);
                let mut layers = init_layers(memory, active_layers, dispcnt.bg_mode);

                match dispcnt.bg_mode {
                    0 => {
                        // 0 - text
                        // 1 - text
                        // 2 - text
                        // 3 - text
                        bgm0_draw_scanline(memory, screen_y, &mut layers)
                    }

                    1 => {
                        // 0 - text
                        // 1 - text
                        // 2 - affine

                        println!("BGM 1 not implemented");
                        bgm1_draw_scanline(memory, screen_y, &mut layers)
                    }

                    2 => {
                        // 2 - affine
                        // 3 - affine

                        println!("BGM 2 not implemented");
                        [0; HDRAW_PIXELS as usize].to_vec()
                    }

                    _ => {
                        unreachable!();
                    }
                }
            }

            // bitmap modes
            3..=5 => {
                let selected_frame_addr = if dispcnt.frame_select as u8 == 0 {
                    // frame 0
                    VRAM_START
                } else {
                    // frame 1
                    VRAM_START + 0xA000
                };

                match dispcnt.bg_mode {
                    3 => bgm3_draw_scanline(memory, screen_y),

                    4 => bgm4_draw_scanline(memory, screen_y, selected_frame_addr),

                    5 => bgm5_draw_scanline(memory, screen_y, selected_frame_addr),

                    _ => {
                        unreachable!()
                    }
                }
            }

            _ => {
                tracing::error!("Invalid background mode: {}", dispcnt.bg_mode);
                [0; HDRAW_PIXELS as usize].to_vec()
            }
        };

        for i in 0..(HDRAW_PIXELS as u32) {
            let arr_index = (screen_y as u32 * HDRAW_PIXELS as u32 + i) as usize;

            self.display.screen[arr_index] = scanline[i as usize];
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
