use crate::system_ctrl_registers::WAITCNT_ADDR;

pub fn align16(addr: u32) -> u32 {
    addr & !1u32
}

pub fn align32(addr: u32) -> u32 {
    addr & !3u32
}

pub fn touches_waitcnt(addr: u32, width: u8) -> bool {
    addr < WAITCNT_ADDR + 2 && WAITCNT_ADDR < addr + width as u32
}
