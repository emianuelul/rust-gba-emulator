pub fn align16(addr: u32) -> u32 {
    addr & !1u32
}

pub fn align32(addr: u32) -> u32 {
    addr & !3u32
}

pub fn touches_addr(check: u32, addr: u32, width: u8) -> bool {
    addr < check + 2 && check < addr + width as u32
}

pub fn is_imc(addr: u32) -> bool {
    (addr & 0xFF000000) == 0x04000000 && (0x800..=0x803).contains(&(addr & 0xFFFF))
}

pub fn mul_get_m(value: u32, signed: bool) -> u32 {
    let count = if signed {
        value.leading_ones().max(value.leading_zeros())
    } else {
        value.leading_zeros()
    };

    match count {
        24..=32 => 1,
        16..24 => 2,
        8..16 => 3,
        _ => 4,
    }
}
