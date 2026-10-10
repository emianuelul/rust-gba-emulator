use crate::memory_area::GBAMemory;

fn get_mem_reg16_bit(data: u16, pos: usize) -> u8 {
    ((data >> pos) & 1) as u8
}

fn get_mem_reg32_bit(data: u32, pos: usize) -> u8 {
    ((data >> pos) & 1) as u8
}

pub fn get_register16_field_value(register: u16, (pos, len): (u8, u8)) -> u16 {
    let mut field_value: u16 = 0;
    for i in (0..len).rev() {
        let offset = pos + i;
        let bit = get_mem_reg16_bit(register, offset as usize);
        field_value = (field_value << 1) | (bit as u16);
    }
    field_value
}

pub fn get_register32_field_value(register: u32, (pos, len): (u8, u8)) -> u32 {
    let mut field_value: u32 = 0;
    for i in (0..len).rev() {
        let offset = pos + i;
        let bit = get_mem_reg32_bit(register, offset as usize);
        field_value = (field_value << 1) | (bit as u32);
    }
    field_value
}

pub fn set_register16_field_value(register: u16, (pos, len): (u8, u8), value: u16) -> u16 {
    let mut result = register;
    for i in 0..16 {
        let is_in_range = i >= pos && i < pos + len;
        if is_in_range {
            let value_bit = (value >> ((i - pos) % 16)) & 1;
            if value_bit == 1 {
                result |= value_bit << i;
            } else {
                result &= !(1 << i);
            }
        }
    }

    result
}

pub fn set_register32_field_value(register: u32, (pos, len): (u8, u8), value: u32) -> u32 {
    let mut result = register;
    for i in 0..32 {
        let is_in_range = i >= pos && i < pos + len;
        if is_in_range {
            let value_bit = (value >> ((i - pos) % 32)) & 1;
            if value_bit == 1 {
                result |= value_bit << i;
            } else {
                result &= !(1 << i);
            }
        }
    }

    result
}

pub const DISPCNT_ADDR: u32 = 0x04000000;

pub const DISPCNT_BG_MODE: (u8, u8) = (0, 3);
pub const DISPCNT_CGB_MODE: (u8, u8) = (3, 1);
pub const DISPCNT_FRAME_SELECT: (u8, u8) = (4, 1);
pub const DISPCNT_H_BLANK_INTERVAL: (u8, u8) = (5, 1);
pub const DISPCNT_OBJ_CHAR_VRAM_MAPPING: (u8, u8) = (6, 1);
pub const DISPCNT_FORCED_BLANK: (u8, u8) = (7, 1);
pub const DISPCNT_DISP_BG0: (u8, u8) = (8, 1);
pub const DISPCNT_DISP_BG1: (u8, u8) = (9, 1);
pub const DISPCNT_DISP_BG2: (u8, u8) = (10, 1);
pub const DISPCNT_DISP_BG3: (u8, u8) = (11, 1);
pub const DISPCNT_DISP_OBJ: (u8, u8) = (12, 1);
pub const DISPCNT_WINDOW0: (u8, u8) = (13, 1);
pub const DISPCNT_WINDOW1: (u8, u8) = (14, 1);
pub const DISPCNT_OBJ_WINDOW: (u8, u8) = (15, 1);

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

pub const DISPSTAT_VBLANK_FLAG: (u8, u8) = (0, 1);
pub const DISPSTAT_HBLANK_FLAG: (u8, u8) = (1, 1);
pub const DISPSTAT_VCOUNTER_FLAG: (u8, u8) = (2, 1);
pub const DISPSTAT_VBLANK_IRQ_ENABLE: (u8, u8) = (3, 1);
pub const DISPSTAT_HBLANK_IRQ_ENABLE: (u8, u8) = (4, 1);
pub const DISPSTAT_VCOUNTER_IRQ_ENABLE: (u8, u8) = (5, 1);
pub const DISPSTAT_VCOUNT_SETTING: (u8, u8) = (8, 8);

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

// getters
impl GBAMemory {
    pub fn get_dispcnt(&self) -> DISPCNT {
        let raw_value = self.read16(DISPCNT_ADDR).0;

        let bg_mode = get_register16_field_value(raw_value, DISPCNT_BG_MODE) as u8;
        let cgb_mode = get_register16_field_value(raw_value, DISPCNT_CGB_MODE) == 1;
        let frame_select = get_register16_field_value(raw_value, DISPCNT_FRAME_SELECT) == 1;
        let h_blank_interval = get_register16_field_value(raw_value, DISPCNT_H_BLANK_INTERVAL) == 1;
        let obj_char_vram_mapping =
            get_register16_field_value(raw_value, DISPCNT_OBJ_CHAR_VRAM_MAPPING) == 1;
        let forced_blank = get_register16_field_value(raw_value, DISPCNT_FORCED_BLANK) == 1;
        let disp_bg0 = get_register16_field_value(raw_value, DISPCNT_DISP_BG0) == 1;
        let disp_bg1 = get_register16_field_value(raw_value, DISPCNT_DISP_BG1) == 1;
        let disp_bg2 = get_register16_field_value(raw_value, DISPCNT_DISP_BG2) == 1;
        let disp_bg3 = get_register16_field_value(raw_value, DISPCNT_DISP_BG3) == 1;
        let disp_obj = get_register16_field_value(raw_value, DISPCNT_DISP_OBJ) == 1;
        let window0 = get_register16_field_value(raw_value, DISPCNT_WINDOW0) == 1;
        let window1 = get_register16_field_value(raw_value, DISPCNT_WINDOW1) == 1;
        let obj_window = get_register16_field_value(raw_value, DISPCNT_OBJ_WINDOW) == 1;

        DISPCNT {
            bg_mode,
            cgb_mode,
            frame_select,
            h_blank_interval,
            obj_char_vram_mapping,
            forced_blank,
            disp_bg0,
            disp_bg1,
            disp_bg2,
            disp_bg3,
            disp_obj,
            window0,
            window1,
            obj_window,
        }
    }

    pub fn get_greenswap(&self) -> GREENSWAP {
        let raw_value = self.read16(GREENSWAP_ADDR).0;

        let green_swap_toggle =
            get_register16_field_value(raw_value, GREENSWAP_GREEN_SWAP_TOGGLE) == 1;

        GREENSWAP { green_swap_toggle }
    }

    pub fn get_dispstat(&self) -> DISPSTAT {
        let register = self.read16(DISPSTAT_ADDR).0;

        let vblank_flag = get_register16_field_value(register, DISPSTAT_VBLANK_FLAG) == 1;
        let hblank_flag = get_register16_field_value(register, DISPSTAT_HBLANK_FLAG) == 1;
        let vcounter_flag = get_register16_field_value(register, DISPSTAT_VCOUNTER_FLAG) == 1;
        let vblank_irq_enable =
            get_register16_field_value(register, DISPSTAT_VBLANK_IRQ_ENABLE) == 1;
        let hblank_irq_enable =
            get_register16_field_value(register, DISPSTAT_HBLANK_IRQ_ENABLE) == 1;
        let vcounter_irq_enable =
            get_register16_field_value(register, DISPSTAT_VCOUNTER_IRQ_ENABLE) == 1;
        let vcount_setting = get_register16_field_value(register, DISPSTAT_VCOUNT_SETTING) as u8;

        DISPSTAT {
            vblank_flag,
            hblank_flag,
            vcounter_flag,
            vblank_irq_enable,
            hblank_irq_enable,
            vcounter_irq_enable,
            vcount_setting,
        }
    }

    pub fn get_vcount(&self) -> VCOUNT {
        let register = self.read16(VCOUNT_ADDR).0;

        let current_scanline = get_register16_field_value(register, VCOUNT_CURRENT_SCANLINE) as u8;

        VCOUNT { current_scanline }
    }

    pub fn get_bgxcnt(&self, which: u8) -> BGXCNT {
        let addr = BGXCNT_ADDR.wrapping_add((which as u32).wrapping_mul(2));
        let register = self.read16(addr).0;

        let bg_priority = get_register16_field_value(register, BGXCNT_BG_PRIORITY) as u8;
        let char_base_block = get_register16_field_value(register, BGXCNT_CHAR_BASE_BLOCK) as u8;
        let mosaic = get_register16_field_value(register, BGXCNT_MOSAIC) == 1;
        let colors_palettes = get_register16_field_value(register, BGXCNT_COLORS_PALETTES) == 1;
        let screen_base_block =
            get_register16_field_value(register, BGXCNT_SCREEN_BASE_BLOCK) as u8;
        let display_area_overflow =
            get_register16_field_value(register, BGXCNT_DISPLAY_AREA_OVERFLOW) == 1;
        let screen_size = get_register16_field_value(register, BGXCNT_SCREEN_SIZE) as u8;

        BGXCNT {
            bg_priority,
            char_base_block,
            mosaic,
            colors_palettes,
            screen_base_block,
            display_area_overflow,
            screen_size,
        }
    }

    pub fn get_bgxofs(&self, which: u8, horizontal: bool) -> BGXOFS {
        let addr = if horizontal {
            BGXHOFS_ADDR.wrapping_add((which as u32).wrapping_mul(4))
        } else {
            BGXVOFS_ADDR.wrapping_add((which as u32).wrapping_mul(4))
        };
        let register = self.read16(addr).0;

        let offset = get_register16_field_value(register, BGXOFS_OFFSET);

        BGXOFS { offset }
    }
}

// Setters
impl GBAMemory {
    pub fn set_dispcnt(&mut self, other: DISPCNT) {
        let mut register: u16 = 0;

        register = set_register16_field_value(register, DISPCNT_BG_MODE, other.bg_mode as u16);
        register = set_register16_field_value(register, DISPCNT_CGB_MODE, other.cgb_mode as u16);
        register =
            set_register16_field_value(register, DISPCNT_FRAME_SELECT, other.frame_select as u16);
        register = set_register16_field_value(
            register,
            DISPCNT_H_BLANK_INTERVAL,
            other.h_blank_interval as u16,
        );
        register = set_register16_field_value(
            register,
            DISPCNT_OBJ_CHAR_VRAM_MAPPING,
            other.obj_char_vram_mapping as u16,
        );
        register =
            set_register16_field_value(register, DISPCNT_FORCED_BLANK, other.forced_blank as u16);
        register = set_register16_field_value(register, DISPCNT_DISP_BG0, other.disp_bg0 as u16);
        register = set_register16_field_value(register, DISPCNT_DISP_BG1, other.disp_bg1 as u16);
        register = set_register16_field_value(register, DISPCNT_DISP_BG2, other.disp_bg2 as u16);
        register = set_register16_field_value(register, DISPCNT_DISP_BG3, other.disp_bg3 as u16);
        register = set_register16_field_value(register, DISPCNT_DISP_OBJ, other.disp_obj as u16);
        register = set_register16_field_value(register, DISPCNT_WINDOW0, other.window0 as u16);
        register = set_register16_field_value(register, DISPCNT_WINDOW1, other.window1 as u16);
        register =
            set_register16_field_value(register, DISPCNT_OBJ_WINDOW, other.obj_window as u16);

        self.write16(DISPCNT_ADDR, register);
    }

    pub fn set_greenswap(&mut self, other: GREENSWAP) {
        let mut saved: u16 = 0;

        saved = set_register16_field_value(
            saved,
            GREENSWAP_GREEN_SWAP_TOGGLE,
            other.green_swap_toggle as u16,
        );

        self.write16(GREENSWAP_ADDR, saved);
    }

    pub fn set_dispstat(&mut self, other: DISPSTAT) {
        let mut saved: u16 = 0;

        saved = set_register16_field_value(saved, DISPSTAT_VBLANK_FLAG, other.vblank_flag as u16);
        saved = set_register16_field_value(saved, DISPSTAT_HBLANK_FLAG, other.hblank_flag as u16);
        saved =
            set_register16_field_value(saved, DISPSTAT_VCOUNTER_FLAG, other.vcounter_flag as u16);
        saved = set_register16_field_value(
            saved,
            DISPSTAT_VBLANK_IRQ_ENABLE,
            other.vblank_irq_enable as u16,
        );
        saved = set_register16_field_value(
            saved,
            DISPSTAT_HBLANK_IRQ_ENABLE,
            other.hblank_irq_enable as u16,
        );
        saved = set_register16_field_value(
            saved,
            DISPSTAT_VCOUNTER_IRQ_ENABLE,
            other.vcounter_irq_enable as u16,
        );
        saved =
            set_register16_field_value(saved, DISPSTAT_VCOUNT_SETTING, other.vcount_setting as u16);

        self.write16(DISPSTAT_ADDR, saved);
    }

    pub fn set_vcount(&mut self, other: VCOUNT) {
        let mut saved: u16 = 0;

        saved = set_register16_field_value(
            saved,
            VCOUNT_CURRENT_SCANLINE,
            other.current_scanline as u16,
        );

        self.write16(VCOUNT_ADDR, saved);
    }

    pub fn set_bgxcnt(&mut self, which: u8, other: BGXCNT) {
        let mut saved: u16 = 0;

        saved = set_register16_field_value(saved, BGXCNT_BG_PRIORITY, other.bg_priority as u16);
        saved =
            set_register16_field_value(saved, BGXCNT_CHAR_BASE_BLOCK, other.char_base_block as u16);
        saved = set_register16_field_value(saved, BGXCNT_MOSAIC, other.mosaic as u16);
        saved =
            set_register16_field_value(saved, BGXCNT_COLORS_PALETTES, other.colors_palettes as u16);
        saved = set_register16_field_value(
            saved,
            BGXCNT_SCREEN_BASE_BLOCK,
            other.screen_base_block as u16,
        );
        saved = set_register16_field_value(
            saved,
            BGXCNT_DISPLAY_AREA_OVERFLOW,
            other.display_area_overflow as u16,
        );
        saved = set_register16_field_value(saved, BGXCNT_SCREEN_SIZE, other.screen_size as u16);

        let addr = BGXCNT_ADDR.wrapping_add((which as u32).wrapping_mul(2));
        self.write16(addr, saved);
    }

    pub fn set_bgxofs(&mut self, which: u8, horizontal: bool, other: BGXOFS) {
        let mut saved: u16 = 0;

        saved = set_register16_field_value(saved, BGXOFS_OFFSET, other.offset);

        let addr = if horizontal {
            BGXHOFS_ADDR.wrapping_add((which as u32).wrapping_mul(4))
        } else {
            BGXVOFS_ADDR.wrapping_add((which as u32).wrapping_mul(4))
        };

        self.write16(addr, saved);
    }
}
