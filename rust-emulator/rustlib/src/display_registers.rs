use crate::memory_area::GBAMemory;

fn get_mem_reg16_bit(data: u16, pos: usize) -> u8 {
    ((data >> pos) & 1) as u8
}

fn get_mem_reg32_bit(data: u32, pos: usize) -> u8 {
    ((data >> pos) & 1) as u8
}

impl GBAMemory {
    pub fn get_register16_field_value(&self, register: u16, (pos, len): (u8, u8)) -> u16 {
        let mut field_value: u16 = 0;
        for i in (0..len).rev() {
            let offset = pos + i;
            let bit = get_mem_reg16_bit(register, offset as usize);
            field_value = (field_value << 1) | (bit as u16);
        }
        field_value
    }

    pub fn get_register32_field_value(&self, register: u32, (pos, len): (u8, u8)) -> u32 {
        let mut field_value: u32 = 0;
        for i in (0..len).rev() {
            let offset = pos + i;
            let bit = get_mem_reg32_bit(register, offset as usize);
            field_value = (field_value << 1) | (bit as u32);
        }
        field_value
    }

    pub fn set_register16_field_value(&mut self, addr: u32, (pos, len): (u8, u8), value: u16) {
        let register = self.read16(addr).0;
        let mut result: u16 = register;

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

        self.write16(addr, result);
    }

    pub fn set_register32_field_value(&mut self, addr: u32, (pos, len): (u8, u8), value: u32) {
        let register = self.read32(addr).0;
        let mut result: u32 = register;

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

        self.write32(addr, result);
    }
}

pub const DISPCNT_ADDR: u32 = 0x04000000;
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
pub struct GREENSWAP {
    pub green_swap_toggle: bool,
}

pub const DISPSTAT_ADDR: u32 = 0x04000004;
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
pub struct VCOUNT {
    pub current_scanline: u8,
}

pub const BGXCNT_ADDR: u32 = 0x04000008;
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
pub struct BGXOFS {
    pub offset: u16,
}

impl GBAMemory {
    pub fn get_dispcnt(&self) -> DISPCNT {
        let raw_value = self.read16(DISPCNT_ADDR).0;

        let bg_mode = self.get_register16_field_value(raw_value, (0, 3)) as u8;
        let cgb_mode = self.get_register16_field_value(raw_value, (3, 1)) == 1;
        let frame_select = self.get_register16_field_value(raw_value, (4, 1)) == 1;
        let h_blank_interval = self.get_register16_field_value(raw_value, (5, 1)) == 1;
        let obj_char_vram_mapping = self.get_register16_field_value(raw_value, (6, 1)) == 1;
        let forced_blank = self.get_register16_field_value(raw_value, (7, 1)) == 1;
        let disp_bg0 = self.get_register16_field_value(raw_value, (8, 1)) == 1;
        let disp_bg1 = self.get_register16_field_value(raw_value, (9, 1)) == 1;
        let disp_bg2 = self.get_register16_field_value(raw_value, (10, 1)) == 1;
        let disp_bg3 = self.get_register16_field_value(raw_value, (11, 1)) == 1;
        let disp_obj = self.get_register16_field_value(raw_value, (12, 1)) == 1;
        let window0 = self.get_register16_field_value(raw_value, (13, 1)) == 1;
        let window1 = self.get_register16_field_value(raw_value, (14, 1)) == 1;
        let obj_window = self.get_register16_field_value(raw_value, (15, 1)) == 1;

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

        let green_swap_toggle = self.get_register16_field_value(raw_value, (0, 1)) == 1;

        GREENSWAP { green_swap_toggle }
    }

    pub fn get_dispstat(&self) -> DISPSTAT {
        let register = self.read16(DISPSTAT_ADDR).0;

        let vblank_flag = self.get_register16_field_value(register, (0, 1)) == 1;
        let hblank_flag = self.get_register16_field_value(register, (1, 1)) == 1;
        let vcounter_flag = self.get_register16_field_value(register, (2, 1)) == 1;
        let vblank_irq_enable = self.get_register16_field_value(register, (3, 1)) == 1;
        let hblank_irq_enable = self.get_register16_field_value(register, (4, 1)) == 1;
        let vcounter_irq_enable = self.get_register16_field_value(register, (5, 1)) == 1;
        let vcount_setting = self.get_register16_field_value(register, (8, 8)) as u8;

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

        let current_scanline = self.get_register16_field_value(register, (0, 8)) as u8;

        VCOUNT { current_scanline }
    }

    pub fn get_bgxcnt(&self, which: u8) -> BGXCNT {
        let addr = BGXCNT_ADDR.wrapping_add((which as u32).wrapping_mul(2));
        let register = self.read16(addr).0;

        let bg_priority = self.get_register16_field_value(register, (0, 2)) as u8;
        let char_base_block = self.get_register16_field_value(register, (2, 2)) as u8;
        let mosaic = self.get_register16_field_value(register, (6, 1)) == 1;
        let colors_palettes = self.get_register16_field_value(register, (7, 1)) == 1;
        let screen_base_block = self.get_register16_field_value(register, (8, 5)) as u8;
        let display_area_overflow = self.get_register16_field_value(register, (13, 1)) == 1;
        let screen_size = self.get_register16_field_value(register, (14, 2)) as u8;

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

        let offset = self.get_register16_field_value(register, (0, 9));

        BGXOFS { offset }
    }
}
