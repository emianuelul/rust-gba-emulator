use crate::bgmodes_draw::{
    bgm3_draw_scanline, bgm4_draw_scanline, bgm5_draw_scanline, convert_bgr555_rgba,
};
use crate::constants::{
    HDRAW_CYCLES, HDRAW_PIXELS, KB, PRAM_START, SCANLINE_CYCLES, VBLANK_LINES, VDRAW_LINES,
    VRAM_START,
};
use crate::display_registers::{
    DISPCNT_ADDR, DISPSTAT_ADDR, RegBgxcnt, RegDispcnt, RegDispstat, RegTileData, RegVcount,
    VCOUNT_ADDR, get_bgxcnt, get_bgxhofs, get_bgxvofs,
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

struct BGLayer {
    bg_index: usize,
    cnt: RegBgxcnt,
    xy: (u16, u16),
    sc_addr: u32,
    width: u16,
    height: u16,
    scanline: Vec<Option<u16>>,
}

impl PPU {
    fn draw_scanline(&mut self, memory: &mut GBAMemory, screen_y: u8) {
        let dispcnt: RegDispcnt = memory.read16(DISPCNT_ADDR).0.into();
        match dispcnt.bg_mode {
            // tile modes
            0..=2 => {
                let mut active_layers: Vec<u8> = Vec::new();
                if dispcnt.disp_bg0 {
                    active_layers.push(0);
                }
                if dispcnt.disp_bg1 {
                    active_layers.push(1);
                }
                if dispcnt.disp_bg2 {
                    active_layers.push(2);
                }
                if dispcnt.disp_bg3 {
                    active_layers.push(3);
                }

                let mut layers: Vec<BGLayer> = Vec::new();
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

                    layers.push(BGLayer {
                        bg_index: active_layer as usize,
                        cnt,
                        xy,
                        sc_addr,
                        width,
                        height,
                        scanline: Vec::new(),
                    });
                }

                match dispcnt.bg_mode {
                    0 => {
                        for layer in layers.iter_mut() {
                            for screen_x in 0..HDRAW_PIXELS {
                                // convert from screen coords to map coords
                                let map_x = (screen_x as u16 + layer.xy.0) % layer.width;
                                let map_y = (screen_y as u16 + layer.xy.1) % layer.height;

                                // tile coords
                                let mut tile_x: u32 = map_x as u32 / 8;
                                let tile_pixel_x = map_x % 8;
                                let mut tile_y: u32 = map_y as u32 / 8;
                                let tile_pixel_y = map_y % 8;

                                // get tile index
                                let block_offset_x: u32 = if tile_x >= 32 {
                                    1
                                } else {
                                    0
                                };
                                let block_offset_y: u32 = if tile_y >= 32 {
                                    2
                                } else {
                                    0
                                };
                                let block_offset = block_offset_y + block_offset_x;

                                tile_x %= 32;
                                tile_y %= 32;

                                let screenblock_addr = layer.sc_addr + block_offset * 2 * KB;
                                let tile_index = tile_y * 32 + tile_x;
                                let tile_addr = screenblock_addr + tile_index * 2;

                                // read tile data
                                let tile_data: RegTileData = memory.read16(tile_addr).0.into();

                                // convert from tile data to pixel and color
                                let color_depth_bytes: u32 = if layer.cnt.colors_palettes {
                                    8
                                } else {
                                    4
                                };
                                let tile_pixels_addr = VRAM_START
                                    + layer.cnt.char_base_block as u32 * 16 * KB
                                    + tile_data.tile_number as u32 * color_depth_bytes * 8;

                                let curr_x = if tile_data.hflip {
                                    7 - tile_pixel_x
                                } else {
                                    tile_pixel_x
                                };
                                let curr_y = if tile_data.vflip {
                                    7 - tile_pixel_y
                                } else {
                                    tile_pixel_y
                                };

                                let color = if layer.cnt.colors_palettes {
                                    let offset = curr_y as u32 * 8 + curr_x as u32;
                                    let idx = memory.read8(tile_pixels_addr + offset).0 as u32;
                                    if idx == 0 {
                                        None
                                    } else {
                                        Some(memory.read16(PRAM_START + idx * 2).0)
                                    }
                                } else {
                                    let offset = curr_y as u32 * 4 + curr_x as u32 / 2;
                                    let byte = memory.read8(tile_pixels_addr + offset).0 as u32;
                                    let nib = if curr_x.is_multiple_of(2) {
                                        byte & 0x0F
                                    } else {
                                        byte >> 4
                                    };
                                    if nib == 0 {
                                        None
                                    } else {
                                        let idx = tile_data.palette_number as u32 * 16 + nib;
                                        Some(memory.read16(PRAM_START + idx * 2).0)
                                    }
                                };
                                layer.scanline.push(color);
                            }
                        }

                        // combinefor each scanline
                        let mut final_scanline: Vec<Option<u32>> =
                            vec![None; HDRAW_PIXELS as usize];
                        layers.sort_by_key(|a| (a.cnt.bg_priority, a.bg_index));

                        for layer in layers.iter() {
                            (0..HDRAW_PIXELS as usize).for_each(|i| {
                                if final_scanline[i].is_none()
                                    && let Some(bgr) = layer.scanline[i]
                                {
                                    final_scanline[i] = Some(convert_bgr555_rgba(bgr));
                                }
                            });
                        }

                        let backdrop = convert_bgr555_rgba(memory.read16(PRAM_START).0);
                        (0..HDRAW_PIXELS as usize).for_each(|i| {
                            let idx = screen_y as usize * HDRAW_PIXELS as usize + i;
                            self.display.screen[idx] = final_scanline[i].unwrap_or(backdrop);
                        });
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

            // bitmap modes
            3..=5 => {
                let selected_frame_addr = if dispcnt.frame_select as u8 == 0 {
                    // frame 0
                    VRAM_START
                } else {
                    // frame 1
                    VRAM_START + 0xA000
                };

                let scanline = match dispcnt.bg_mode {
                    3 => bgm3_draw_scanline(memory, screen_y),

                    4 => bgm4_draw_scanline(memory, screen_y, selected_frame_addr),

                    5 => bgm5_draw_scanline(memory, screen_y, selected_frame_addr),

                    _ => {
                        unreachable!()
                    }
                };

                for i in 0..(HDRAW_PIXELS as u32) {
                    let arr_index = (screen_y as u32 * HDRAW_PIXELS as u32 + i) as usize;

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
