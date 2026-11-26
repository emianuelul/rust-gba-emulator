use crate::{
    constants::{HDRAW_PIXELS, PRAM_START, VRAM_START},
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

pub fn bgm3_draw_scanline(memory: &mut GBAMemory, scanline_index: u8) -> [u32; 260] {
    // MODE 3 - 15bit colors, no pallette ram
    // layer 2
    let mut scanline: [u32; 260] = [0; 260];
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
) -> [u32; 260] {
    // MODE 4 - 8bit colors, 256/1 palette
    // layer 2
    let mut scanline: [u32; 260] = [0; 260];
    for i in 0..(HDRAW_PIXELS as u32) {
        let arr_index = (scanline_index as u32 * HDRAW_PIXELS as u32 + i) as usize;
        let addr = selected_frame_addr + (arr_index as u32);
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
) -> [u32; 260] {
    // MODE 5 - 16bit colors, 160x128, no pallette ram
    // layer 2

    let mut scanline: [u32; 260] = [0; 260];

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
