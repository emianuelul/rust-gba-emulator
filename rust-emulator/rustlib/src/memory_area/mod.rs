mod reads;
mod writes;

use crate::system_ctrl_registers::{IMC_ADDR, WAITCNT_ADDR};
use std::cell::Cell;

struct InternalMemory {
    bios: Vec<u8>, // 16Kb      0x00000000 - 0x00003FFF
    // unused ~32Mb     0x00004000 - 0x01FFFFFF
    wram_on_board: Vec<u8>, // 256Kb     0x02000000 - 0x020FFFF
    // unused ~15.75Mb  0x02040000 - 0x02FFFFFF
    wram_on_chip: Vec<u8>, // 32Kb      0x03000000 - 0x03007FFF
    // unused ~16Mb     0x03008000 - 0x03FFFFFF
    io_registers: Vec<u8>, // 1024bytes 0x04000000 - 0x040003FE
    // unused ~16Mb     0x04000400 - 0x04FFFFFF
    imc: Vec<u8>,
}

struct DisplayMemory {
    palette_ram: Vec<u8>, // 1Kb       0x05000000 - 0x050003FF
    // ~16Mb     0x05000400 - 0x05FFFFFF
    vram: Vec<u8>, // 96Kb      0x06000000 - 0x06017FFF
    // ~15.9Mb   0x06018000 - 0x06FFFFFF
    oam: Vec<u8>, // 1Kb       0x07000000 - 0x070003FF
                  // ~16Mb     0x07000400 - 0x07FFFFFF
}

struct ExternalMemory {
    rom: Vec<u8>, // wait0 32Mb      0x08000000 - 0x09FFFFFF mirrored wait1 32Mb           0x0A000000 - 0x0BFFFFFF
    // mirrored wait2 32Mb           0x0C000000 - 0x0DFFFFFF
    sram: Vec<u8>, // 64Kb           0x0E000000 - 0x0E00FFFF
                   // unused ~32Mb   0x0E010000 - 0x0FFFFFFF
}

pub struct Cache {
    pub ns0_wait: (u32, u32),
    pub ns1_wait: (u32, u32),
    pub ns2_wait: (u32, u32),
    pub sram_wait: u32,
    pub use_ewram: bool,
    pub ewram_wait: u32,
}

pub struct GBAMemory {
    last_access: Cell<(u32, u8)>,
    internal: InternalMemory,
    display: DisplayMemory,
    external: ExternalMemory,
    pub cache: Cache,
}

// INIT
impl InternalMemory {
    fn new() -> Self {
        InternalMemory {
            bios: [0; 16 * 1024].to_vec(),
            wram_on_board: [0; 256 * 1024].to_vec(),
            wram_on_chip: [0; 32 * 1024].to_vec(),
            io_registers: [0; 1024].to_vec(),
            imc: [0; 4].to_vec(),
        }
    }
}

impl DisplayMemory {
    fn new() -> Self {
        DisplayMemory {
            palette_ram: [0; 1024].to_vec(),
            vram: [0; 96 * 1024].to_vec(),
            oam: [0; 1024].to_vec(),
        }
    }
}

impl ExternalMemory {
    fn new(rom_data: Vec<u8>) -> Self {
        ExternalMemory {
            rom: rom_data,
            sram: [0; 64 * 1024].to_vec(),
        }
    }
}

impl Cache {
    fn new() -> Self {
        Cache {
            ns0_wait: (0, 0),
            ns1_wait: (0, 0),
            ns2_wait: (0, 0),
            sram_wait: 0,
            ewram_wait: 0,
            use_ewram: false,
        }
    }
}

impl GBAMemory {
    fn get_rom_mirror_clk(&self, addr: u32, bytes: u8) -> u32 {
        let sequential: bool = addr == self.last_access.get().0 + self.last_access.get().1 as u32
            && !addr.is_multiple_of(128 * 1024);

        let wait = (addr >> 25) & 3;
        let (n, s) = match wait {
            0 => self.cache.ns0_wait,

            1 => self.cache.ns1_wait,

            2 => self.cache.ns2_wait,

            _ => {
                unreachable!()
            }
        };

        if sequential {
            if bytes == 4 {
                s * 2
            } else {
                s
            }
        } else {
            if bytes == 4 {
                n + s
            } else {
                n
            }
        }
    }
}

const TEMP_WAITCNT: u16 = 0x4317;
const TEMP_IMC: u32 = 0x0D000020;
impl GBAMemory {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let mut returned = GBAMemory {
            last_access: (0, 0).into(),
            internal: InternalMemory::new(),
            display: DisplayMemory::new(),
            external: ExternalMemory::new(rom_data),
            cache: Cache::new(),
        };

        // REMOVE: AFTER BIOS INIT
        returned.write16(WAITCNT_ADDR, TEMP_WAITCNT);
        returned.write32(IMC_ADDR, TEMP_IMC);

        returned
    }

    pub fn get_rom_size(&self) -> usize {
        self.external.rom.len()
    }

    pub fn get_rom_end_addr(&self) -> usize {
        0x08000000 + self.external.rom.len()
    }
}
