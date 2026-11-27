use crate::{
    constants::{HDRAW_PIXELS, KB, PRAM_START, VRAM_START},
    display_registers::{RegBgxcnt, RegTileData},
    memory_area::GBAMemory,
};

// COLOR INDEX 0 IN PRAM IS CONSIDERED FULLY TRANSPARENT

fn convert_bgr555_rgba(bgr: u16) -> u32 {
    let r = 0b11111 & bgr;
    let g = 0b11111 & (bgr >> 5);
    let b = 0b11111 & (bgr >> 10);

    let r8 = ((r << 3) | (r >> 2)) as u32;
    let g8 = ((g << 3) | (g >> 2)) as u32;
    let b8 = ((b << 3) | (b >> 2)) as u32;

    (0xFF << 24) | (b8 << 16) | (g8 << 8) | r8
}

pub struct BGLayer {
    pub bg_index: usize,
    pub cnt: RegBgxcnt,
    pub xy: (u16, u16),
    pub sc_addr: u32,
    pub width: u16,
    pub height: u16,
    pub scanline: Vec<Option<u16>>,
    pub kind: bool, // 0 - text; 1 - affine
}

fn render_text_scanline(memory: &GBAMemory, layer: &mut BGLayer, screen_y: u8) {
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

fn render_affine_scanline(memory: &GBAMemory, screen_y: u8, layer: &mut BGLayer) {}

fn get_combined_layers(layers: &mut [BGLayer]) -> Vec<Option<u32>> {
    let mut final_scanline = vec![None; HDRAW_PIXELS as usize];
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

    final_scanline
}

fn get_output_scanline(backdrop: u32, combined: Vec<Option<u32>>) -> Vec<u32> {
    let mut output = vec![0u32; HDRAW_PIXELS as usize];
    for i in 0..HDRAW_PIXELS as usize {
        output[i] = combined[i].unwrap_or(backdrop);
    }
    output
}

pub fn bgm0_draw_scanline(memory: &GBAMemory, screen_y: u8, layers: &mut [BGLayer]) -> Vec<u32> {
    for layer in layers.iter_mut() {
        render_text_scanline(memory, layer, screen_y);
    }

    let combined_scanline = get_combined_layers(layers);
    let backdrop = convert_bgr555_rgba(memory.read16(PRAM_START).0);

    get_output_scanline(backdrop, combined_scanline)
}

pub fn bgm1_draw_scanline(
    memory: &mut GBAMemory,
    screen_y: u8,
    layers: &mut [BGLayer],
) -> Vec<u32> {
    for layer in layers.iter_mut() {
        if layer.kind {
            render_affine_scanline(memory, screen_y, layer);
        } else {
            render_text_scanline(memory, layer, screen_y);
        }
    }

    let combined_scanline = get_combined_layers(layers);
    let backdrop = convert_bgr555_rgba(memory.read16(PRAM_START).0);

    get_output_scanline(backdrop, combined_scanline)
}

pub fn bgm3_draw_scanline(memory: &mut GBAMemory, scanline_index: u8) -> Vec<u32> {
    // MODE 3 - 15bit colors, no pallette ram
    let mut scanline = vec![0u32; HDRAW_PIXELS as usize];
    for i in 0..(HDRAW_PIXELS as u32) {
        let arr_index = (scanline_index as u32 * HDRAW_PIXELS as u32 + i) as usize;
        let addr = VRAM_START + (arr_index as u32) * 2;
        let pixel = memory.read16(addr).0;
        scanline[i as usize] = convert_bgr555_rgba(pixel);
    }
    scanline
}

pub fn bgm4_draw_scanline(
    memory: &mut GBAMemory,
    scanline_index: u8,
    selected_frame_addr: u32,
) -> Vec<u32> {
    // MODE 4 - 8bit colors, 256/1 palette
    let mut scanline = vec![0u32; HDRAW_PIXELS as usize];
    for i in 0..(HDRAW_PIXELS as u32) {
        let arr_index = scanline_index as u32 * HDRAW_PIXELS as u32 + i;
        let addr = selected_frame_addr + arr_index;
        let pram_index = memory.read8(addr).0 as u32;
        let pixel = memory.read16(PRAM_START + pram_index * 2).0;
        scanline[i as usize] = convert_bgr555_rgba(pixel);
    }
    scanline
}

pub fn bgm5_draw_scanline(
    memory: &mut GBAMemory,
    scanline_index: u8,
    selected_frame_addr: u32,
) -> Vec<u32> {
    // MODE 5 - 16bit colors, 160x128, no pallette ram
    let mut scanline = vec![0u32; HDRAW_PIXELS as usize];
    let backdrop = convert_bgr555_rgba(memory.read16(PRAM_START).0);
    for i in 0..(HDRAW_PIXELS as u32) {
        let color = if i < 160 && scanline_index < 128 {
            let addr = selected_frame_addr + (scanline_index as u32 * 160 + i) * 2;
            convert_bgr555_rgba(memory.read16(addr).0)
        } else {
            backdrop
        };
        scanline[i as usize] = color;
    }
    scanline
}
