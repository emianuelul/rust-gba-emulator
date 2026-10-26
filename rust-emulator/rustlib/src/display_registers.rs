use crate::register;

pub const DISPCNT_ADDR: u32 = 0x04000000;
pub const DISPCNT_BG_MODE: (usize, usize) = (0, 3);
pub const DISPCNT_CGB_MODE: (usize, usize) = (3, 1);
pub const DISPCNT_FRAME_SELECT: (usize, usize) = (4, 1);
pub const DISPCNT_H_BLANK_INTERVAL: (usize, usize) = (5, 1);
pub const DISPCNT_OBJ_CHAR_VRAM_MAPPING: (usize, usize) = (6, 1);
pub const DISPCNT_FORCED_BLANK: (usize, usize) = (7, 1);
pub const DISPCNT_DISP_BG0: (usize, usize) = (8, 1);
pub const DISPCNT_DISP_BG1: (usize, usize) = (9, 1);
pub const DISPCNT_DISP_BG2: (usize, usize) = (10, 1);
pub const DISPCNT_DISP_BG3: (usize, usize) = (11, 1);
pub const DISPCNT_DISP_OBJ: (usize, usize) = (12, 1);
pub const DISPCNT_WINDOW0: (usize, usize) = (13, 1);
pub const DISPCNT_WINDOW1: (usize, usize) = (14, 1);
pub const DISPCNT_OBJ_WINDOW: (usize, usize) = (15, 1);
register!(
    DISPCNT: u16 {
        bg_mode: u8 => DISPCNT_BG_MODE,
        cgb_mode: bool => DISPCNT_CGB_MODE,
        frame_select: bool => DISPCNT_FRAME_SELECT,
        h_blank_interval: bool => DISPCNT_H_BLANK_INTERVAL,
        obj_char_vram_mapping: bool => DISPCNT_OBJ_CHAR_VRAM_MAPPING,
        forced_blank: bool => DISPCNT_FORCED_BLANK,
        disp_bg0: bool => DISPCNT_DISP_BG0,
        disp_bg1: bool => DISPCNT_DISP_BG1,
        disp_bg2: bool => DISPCNT_DISP_BG2,
        disp_bg3: bool => DISPCNT_DISP_BG3,
        disp_obj: bool => DISPCNT_DISP_OBJ,
        window0: bool => DISPCNT_WINDOW0,
        window1: bool => DISPCNT_WINDOW1,
        obj_window: bool => DISPCNT_OBJ_WINDOW,
    }
);

pub const GREENSWAP_ADDR: u32 = 0x04000002;
pub const GREENSWAP_GREEN_SWAP_TOGGLE: (usize, usize) = (0, 1);

register!(
    GREENSWAP: u16 {
         green_swap_toggle: bool => GREENSWAP_GREEN_SWAP_TOGGLE,
    }
);

pub const DISPSTAT_ADDR: u32 = 0x04000004;
pub const DISPSTAT_VBLANK_FLAG: (usize, usize) = (0, 1);
pub const DISPSTAT_HBLANK_FLAG: (usize, usize) = (1, 1);
pub const DISPSTAT_VCOUNTER_FLAG: (usize, usize) = (2, 1);
pub const DISPSTAT_VBLANK_IRQ_ENABLE: (usize, usize) = (3, 1);
pub const DISPSTAT_HBLANK_IRQ_ENABLE: (usize, usize) = (4, 1);
pub const DISPSTAT_VCOUNTER_IRQ_ENABLE: (usize, usize) = (5, 1);
pub const DISPSTAT_VCOUNT_SETTING: (usize, usize) = (8, 8);
register!(
    DISPSTAT: u16 {
        vblank_flag: bool => DISPSTAT_VBLANK_FLAG,
        hblank_flag: bool => DISPSTAT_HBLANK_FLAG,
        vcounter_flag: bool => DISPSTAT_VCOUNTER_FLAG,
        vblank_irq_enable: bool => DISPSTAT_VBLANK_IRQ_ENABLE,
        hblank_irq_enable: bool => DISPSTAT_HBLANK_IRQ_ENABLE,
        vcounter_irq_enable: bool => DISPSTAT_VCOUNTER_IRQ_ENABLE,
        vcount_setting: u8 => DISPSTAT_VCOUNT_SETTING,
    }
);

pub const VCOUNT_ADDR: u32 = 0x04000006;
pub const VCOUNT_CURRENT_SCANLINE: (usize, usize) = (0, 8);
register!(
    VCOUNT: u16 {
         current_scanline: u8 => VCOUNT_CURRENT_SCANLINE,
    }
);

pub const BGXCNT_ADDR: u32 = 0x04000008;
pub const BGXCNT_BG_PRIORITY: (usize, usize) = (0, 2);
pub const BGXCNT_CHAR_BASE_BLOCK: (usize, usize) = (2, 2);
pub const BGXCNT_MOSAIC: (usize, usize) = (6, 1);
pub const BGXCNT_COLORS_PALETTES: (usize, usize) = (7, 1);
pub const BGXCNT_SCREEN_BASE_BLOCK: (usize, usize) = (8, 5);
pub const BGXCNT_DISPLAY_AREA_OVERFLOW: (usize, usize) = (13, 1);
pub const BGXCNT_SCREEN_SIZE: (usize, usize) = (14, 2);
register!(
     BGXCNT: u16 {
         bg_priority: u8 => BGXCNT_BG_PRIORITY,
         char_base_block: u8 => BGXCNT_CHAR_BASE_BLOCK,
         mosaic: bool => BGXCNT_MOSAIC,
         colors_palettes: bool => BGXCNT_COLORS_PALETTES,
         screen_base_block: u8 => BGXCNT_SCREEN_BASE_BLOCK,
         display_area_overflow: bool => BGXCNT_DISPLAY_AREA_OVERFLOW,
         screen_size: u8 => BGXCNT_SCREEN_SIZE,
    }
);

pub const BGXHOFS_ADDR: u32 = 0x04000010;
pub const BGXVOFS_ADDR: u32 = 0x04000012;
pub const BGXOFS_OFFSET: (usize, usize) = (0, 9);
register!(
    BGXOFS: u16 {
        offset: u16 => BGXOFS_OFFSET,
    }
);
