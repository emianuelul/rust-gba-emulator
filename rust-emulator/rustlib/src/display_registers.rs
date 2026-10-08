use crate::memory_area::GBAMemory;

// PPU Utility functions
pub fn get_mem_reg16_bit(data: u16, pos: usize) -> u8 {
    ((data >> pos) & 1) as u8
}

pub fn get_register_field_value(register: u16, (pos, len): (u8, u8)) -> u8 {
    let mut field_value = 0;
    for i in (0..len).rev() {
        let offset = pos + i;
        let bit = get_mem_reg16_bit(register, offset as usize);
        field_value = (field_value << 1) | bit;
    }
    field_value
}

pub fn set_register_field_value(
    memory: &mut GBAMemory,
    register_addr: u32,
    register: u16,
    (pos, len): (u8, u8),
    value: u16,
) {
    let mut result: u16 = register;

    for i in 0..16 {
        let is_in_range = i >= pos && i < pos + len;
        if is_in_range {
            let value_bit = (value >> (i - pos)) & 1;
            if value_bit == 1 {
                result |= value_bit << i;
            } else {
                result &= !(1 << i);
            }
        }
    }

    memory.write16(register_addr, result);
}

pub const DISPCNT_ADDR: u32 = 0x04000000;
pub enum DispcntField {
    BGMode,
    CGBMode,
    DisplayFrameSelect,
    HBlankInterval,
    OBJCharVRAMMapping,
    ForcedBlank,
    Bg0,
    Bg1,
    Bg2,
    Bg3,
    Obj,
    Window0,
    Window1,
    OBJWindow,
}
pub fn get_dispcnt_field(field: DispcntField) -> (u8, u8) {
    match field {
        DispcntField::BGMode => (0, 3),
        DispcntField::CGBMode => (3, 1),
        DispcntField::DisplayFrameSelect => (4, 1),
        DispcntField::HBlankInterval => (5, 1),
        DispcntField::OBJCharVRAMMapping => (6, 1),
        DispcntField::ForcedBlank => (7, 1),
        DispcntField::Bg0 => (8, 1),
        DispcntField::Bg1 => (9, 1),
        DispcntField::Bg2 => (10, 1),
        DispcntField::Bg3 => (11, 1),
        DispcntField::Obj => (12, 1),
        DispcntField::Window0 => (13, 1),
        DispcntField::Window1 => (14, 1),
        DispcntField::OBJWindow => (15, 1),
    }
}

pub const GREENSWAP_ADDR: u32 = 0x04000002;
pub enum GreenswapField {
    GreenSwapToggle,
}
pub fn get_greenswap_field(field: GreenswapField) -> (u8, u8) {
    match field {
        GreenswapField::GreenSwapToggle => (0, 1),
    }
}

pub const DISPSTAT_ADDR: u32 = 0x04000004;
pub enum DispStatField {
    VBlankFlag,
    HBlankFlag,
    VCounterFlag,
    VBlankIRQEnable,
    HBlankIRQEnable,
    VCounterIRQEnable,
    VCountSetting,
}
pub fn get_dispstat_field(field: DispStatField) -> (u8, u8) {
    match field {
        DispStatField::VBlankFlag => (0, 1),
        DispStatField::HBlankFlag => (1, 1),
        DispStatField::VCounterFlag => (2, 1),
        DispStatField::VBlankIRQEnable => (3, 1),
        DispStatField::HBlankIRQEnable => (4, 1),
        DispStatField::VCounterIRQEnable => (5, 1),
        DispStatField::VCountSetting => (8, 8),
    }
}

pub const VCOUNT_ADDR: u32 = 0x04000006;
pub enum VCountField {
    CurrentScanline,
}
pub fn get_vcount_field(field: VCountField) -> (u8, u8) {
    match field {
        VCountField::CurrentScanline => (0, 8),
    }
}

// add x * 2 to obtain correct addr
// ex: BG2CNT = 0x04000008 + 2 * 2 = 0x0400000C
pub const BGXCNT_ADDR: u32 = 0x04000008;
pub enum BGXcntField {
    Priority,
    CharBaseBlock,
    Mosaic,
    ColorsPalettes,
    ScreenBaseBlock,
    DisplayAreaOverflow,
    ScreenSize,
}
pub fn get_bgxcnt_field(field: BGXcntField) -> (u8, u8) {
    match field {
        BGXcntField::Priority => (0, 2),
        BGXcntField::CharBaseBlock => (2, 2),
        BGXcntField::Mosaic => (6, 1),
        BGXcntField::ColorsPalettes => (7, 1),
        BGXcntField::ScreenBaseBlock => (8, 5),
        BGXcntField::DisplayAreaOverflow => (13, 1),
        BGXcntField::ScreenSize => (14, 2),
    }
}

// Same idea as BGXCNT_ADDR, but add x * 4 to either one of those to obtain corrent addr
pub const BGXHOFS_ADDR: u32 = 0x04000010;
pub const BGXVOFS_ADDR: u32 = 0x04000012;
pub enum BGXofsField {
    Offset,
}
pub fn get_bgxofs_field(field: BGXofsField) -> (u8, u8) {
    match field {
        BGXofsField::Offset => (0, 9),
    }
}
