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
        0 => mem.cache.ns0_wait,

        1 => mem.cache.ns1_wait,

        2 => mem.cache.ns2_wait,

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

pub fn is_imc(addr: u32) -> bool {
    (addr & 0xFF000000) == 0x04000000 && (0x800..=0x803).contains(&(addr & 0xFFFF))
}
