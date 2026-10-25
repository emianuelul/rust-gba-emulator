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

pub struct DISPCNT {
    pub bg_mode: u8,
    pub cgb_mode: bool,
    pub frame_select: bool,
    pub h_blank_interval: bool,
    pub obj_char_vram_mapping: bool,
    pub forced_blank: bool,
    pub disp_bg0: bool,
    pub disp_bg1: bool,
    pub disp_bg2: bool,
    pub disp_bg3: bool,
    pub disp_obj: bool,
    pub window0: bool,
    pub window1: bool,
    pub obj_window: bool,
}

pub const GREENSWAP_ADDR: u32 = 0x04000002;
pub const GREENSWAP_GREEN_SWAP_TOGGLE: (u8, u8) = (0, 1);
pub struct GREENSWAP {
    pub green_swap_toggle: bool,
}

pub const DISPSTAT_ADDR: u32 = 0x04000004;
pub const DISPSTAT_VBLANK_FLAG: (usize, usize) = (0, 1);
pub const DISPSTAT_HBLANK_FLAG: (usize, u8) = (1, 1);
pub const DISPSTAT_VCOUNTER_FLAG: (usize, usize) = (2, 1);
pub const DISPSTAT_VBLANK_IRQ_ENABLE: (usize, usize) = (3, 1);
pub const DISPSTAT_HBLANK_IRQ_ENABLE: (usize, usize) = (4, 1);
pub const DISPSTAT_VCOUNTER_IRQ_ENABLE: (usize, usize) = (5, 1);
pub const DISPSTAT_VCOUNT_SETTING: (usize, usize) = (8, 8);
pub struct DISPSTAT {
    pub vblank_flag: bool,
    pub hblank_flag: bool,
    pub vcounter_flag: bool,
    pub vblank_irq_enable: bool,
    pub hblank_irq_enable: bool,
    pub vcounter_irq_enable: bool,
    pub vcount_setting: u8,
}

pub const VCOUNT_ADDR: u32 = 0x04000006;
pub const VCOUNT_CURRENT_SCANLINE: (u8, u8) = (0, 8);
pub struct VCOUNT {
    pub current_scanline: u8,
}

pub const BGXCNT_ADDR: u32 = 0x04000008;
pub const BGXCNT_BG_PRIORITY: (u8, u8) = (0, 2);
pub const BGXCNT_CHAR_BASE_BLOCK: (u8, u8) = (2, 2);
pub const BGXCNT_MOSAIC: (u8, u8) = (6, 1);
pub const BGXCNT_COLORS_PALETTES: (u8, u8) = (7, 1);
pub const BGXCNT_SCREEN_BASE_BLOCK: (u8, u8) = (8, 5);
pub const BGXCNT_DISPLAY_AREA_OVERFLOW: (u8, u8) = (13, 1);
pub const BGXCNT_SCREEN_SIZE: (u8, u8) = (14, 2);
pub struct BGXCNT {
    pub bg_priority: u8,
    pub char_base_block: u8,
    pub mosaic: bool,
    pub colors_palettes: bool,
    pub screen_base_block: u8,
    pub display_area_overflow: bool,
    pub screen_size: u8,
}

pub const BGXHOFS_ADDR: u32 = 0x04000010;
pub const BGXVOFS_ADDR: u32 = 0x04000012;
pub const BGXOFS_OFFSET: (u8, u8) = (0, 9);
pub struct BGXOFS {
    pub offset: u16,
}
