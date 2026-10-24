mod reads;
mod writes;

use crate::system_ctrl_registers::{IMC_ADDR, WAITCNT_ADDR};
use crate::utility_fns::is_imc;
use std::cell::Cell;
use tracing::{error, warn};

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
    fetched_opcode: Cell<u32>,
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
    pub fn set_fetched_opcode(&self, opcode: u32) {
        self.fetched_opcode.set(opcode);
    }

    pub fn is_sequential(&self, addr: u32) -> bool {
        let (last_addr, last_size) = self.last_access.get();
        addr == last_addr.wrapping_add(last_size as u32)
    }

    pub fn set_last_access(&self, addr: u32, bytes: u8) {
        self.last_access.set((addr, bytes));
    }

    pub fn get_access_clk(&self, addr: u32, bytes: u32, seq: bool) -> u32 {
        match addr {
            // BIOS
            0x00000000..=0x00003FFF => 1,

            // EWRAM
            0x02000000..=0x02FFFFFF => {
                if self.cache.use_ewram {
                    if bytes == 4 {
                        self.cache.ewram_wait * 2
                    } else {
                        self.cache.ewram_wait
                    }
                } else {
                    1
                }
            }

            // IWRAM
            0x03000000..=0x03FFFFFF => 1,

            // IO REGISTERS
            0x04000000..=0x040003FF => 1,

            // IMC REGISTER
            x if is_imc(x) => 1,

            // INTERNAL UNUSED MEM AREAS
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => 1,

            // BG OBJ PALETTE RAM
            0x05000000..=0x05FFFFFF => {
                if bytes == 4 {
                    2
                } else {
                    1
                }
            }

            // VRAM
            0x06000000..=0x06FFFFFF => {
                if bytes == 4 {
                    2
                } else {
                    1
                }
            }

            // OAM
            0x07000000..=0x07FFFFFF => 1,

            // GAMEPAK ROM
            0x08000000..=0x0DFFFFFF => {
                let sequential: bool = seq && !addr.is_multiple_of(128 * 1024);

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

            // SRAM
            0x0E000000..=0x0FFFFFFF => self.cache.sram_wait,

            _ => {
                tracing::error!("Memory addr {:x?} is out of bounds", addr);
                0
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
            fetched_opcode: 0.into(),
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
