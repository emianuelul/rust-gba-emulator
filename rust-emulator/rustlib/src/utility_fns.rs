use crate::memory_area::GBAMemory;
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

pub fn get_rom_mirror_clk(addr: u32, mem: &GBAMemory, bytes: u8) -> u32 {
    let wait = (addr >> 25) & 3;
    let ns = match wait {
        0 => mem.ns0_wait,

        1 => mem.ns1_wait,

        2 => mem.ns2_wait,

        _ => {
            unreachable!()
        }
    };

    if bytes == 4 {
        ns.0 + ns.1
    } else {
        ns.0
    }
}
