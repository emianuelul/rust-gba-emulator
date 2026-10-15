use crate::memory_area::GBAMemory;

pub fn align16(addr: u32) -> u32 {
    addr & !1u32
}

pub fn align32(addr: u32) -> u32 {
    addr & !3u32
}

pub fn touches_addr(check: u32, addr: u32, width: u8) -> bool {
    addr < check + 2 && check < addr + width as u32
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
