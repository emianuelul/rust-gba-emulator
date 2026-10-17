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

pub fn get_acces_clk(addr: u32, bytes: u32, seq: bool) -> u32 {
    let mut clk: u32 = 0;
    match addr {
        // EDGECASE
        0x00000000..=0x00003FFF => {
            clk = 1;
        }

        0x02000000..=0x02FFFFFF => {
            if self.cache.use_ewram {
                clk = self.cache.ewram_wait;
            } else {
                clk = 1;
            }
        }

        0x03000000..=0x03FFFFFF => {
            clk = 1;
        }

        // EDGECASE
        0x04000000..=0x040003FF => {
            clk = 1;
        }

        x if is_imc(x) => {
            clk = 1;
        }

        // Unused Mem Areas
        0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
            warn!("Accessing unused memory addr: {:x?}", addr);
            let raw_data = self.fetched_opcode.get();
            data = (raw_data >> ((addr & 3) * 8)) as u8;
            clk = 1;
        }

        _ => {
            error!("Couldn't read addr {:x?} from internal memory", addr)
        }
    }

    clk
}
