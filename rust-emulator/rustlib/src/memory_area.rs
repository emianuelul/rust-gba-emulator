use tracing::{error, warn};

struct InternalMemory {
    bios: Vec<u8>, // 16Kb      0x00000000 - 0x00003FFF
    // unused ~32Mb     0x00004000 - 0x01FFFFFF
    wram_on_board: Vec<u8>, // 256Kb     0x02000000 - 0x0203FFFF
    // unused ~15.75Mb  0x02040000 - 0x02FFFFFF
    wram_on_chip: Vec<u8>, // 32Kb      0x03000000 - 0x03007FFF
    // unused ~16Mb     0x03008000 - 0x03FFFFFF
    io_registers: Vec<u8>, // 1024bytes 0x04000000 - 0x040003FE
                           // unused ~16Mb     0x04000400 - 0x04FFFFFF
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
    rom: Vec<u8>, // wait0 32Mb      0x08000000 - 0x09FFFFFF
    // mirrored wait1 32Mb           0x0A000000 - 0x0BFFFFFF
    // mirrored wait2 32Mb           0x0C000000 - 0x0DFFFFFF
    sram: Vec<u8>, // 64Kb           0x0E000000 - 0x0E00FFFF
                   // unused ~32Mb   0x0E010000 - 0x0FFFFFFF
}

pub struct GBAMemory {
    internal: InternalMemory,
    display: DisplayMemory,
    external: ExternalMemory,
    // unused 0x10000000 - 0xFFFFFFFF
}

impl InternalMemory {
    fn new() -> Self {
        InternalMemory {
            bios: [0; 16 * 1024].to_vec(),
            wram_on_board: [0; 256 * 1024].to_vec(),
            wram_on_chip: [0; 32 * 1024].to_vec(),
            io_registers: [0; 1024].to_vec(),
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

impl GBAMemory {
    pub fn new(rom_data: Vec<u8>) -> Self {
        GBAMemory {
            internal: InternalMemory::new(),
            display: DisplayMemory::new(),
            external: ExternalMemory::new(rom_data),
        }
    }
}

// MEMORY READS
impl InternalMemory {
    fn read8(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;
        // let clk
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                let index = addr as usize;
                match self.bios.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from BIOS", addr),
                }
            }

            0x02000000..=0x02FFFFFF => {
                let index = ((addr - 0x02000000) % self.wram_on_board.len() as u32) as usize;
                match self.wram_on_board.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from On-Board WRAM", addr),
                }
            }

            0x03000000..=0x03FFFFFF => {
                let index = ((addr - 0x03000000) % self.wram_on_chip.len() as u32) as usize;
                match self.wram_on_chip.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from On-Chip WRAM", addr),
                }
            }

            // EDGECASE
            0x04000000..=0x040003FE => {
                let index = (addr - 0x04000000) as usize;
                match self.io_registers.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from I/O Registers", addr),
                }
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                warn!("Accessing unused memory addr: {:x?}", addr);
                // TODO: IMPLEMENT SPECIAL CASE
            }

            _ => {
                error!("Couldn't read addr {:x?} from internal memory", addr)
            }
        }
        data
    }

    fn read16(&self, addr: u32) -> u16 {
        let first = self.read8(addr) as u16;
        let second = self.read8(addr + 1) as u16;

        first | (second << 8)

        // let clk
        //match addr {
        //    // bios
        //    0x00000000..=0x00003FFF => {
        //    }
        //
        //    // wram onboard
        //    0x02000000..=0x02FFFFFF => {
        //    }
        //
        //    // wram onchip
        //    0x03000000..=0x03FFFFFF => {
        //    }
        //
        //    // io registers
        //    0x04000000..=0x040003FE => {
        //    }
        //
        //    // Unused Mem Areas
        //    0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
        //        warn!("Accessing unused memory addr: {:x?}", addr);
        //        // TODO: IMPLEMENT SPECIAL CASE
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from internal memory", addr)
        //    }
        //}
    }

    fn read32(&self, addr: u32) -> u32 {
        let first = self.read16(addr) as u32;
        let second = self.read16(addr + 2) as u32;

        // let clk
        //match addr {
        //    // EDGECASE
        //    0x00000000..=0x00003FFF => {
        //    }
        //
        //    0x02000000..=0x02FFFFFF => {
        //    }
        //
        //    0x03000000..=0x03FFFFFF => {
        //    }
        //
        //    // EDGECASE
        //    0x04000000..=0x040003FE => {
        //    }
        //
        //    // Unused Mem Areas
        //    0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
        //        warn!("Accessing unused memory addr: {:x?}", addr);
        //        // TODO: IMPLEMENT SPECIAL CASE
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from internal memory", addr)
        //    }
        //}

        first | (second << 16)
    }
}

impl DisplayMemory {
    fn read8(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;

        match addr {
            0x05000000..=0x05FFFFFF => {
                let index = ((addr - 0x05000000) % self.palette_ram.len() as u32) as usize;
                match self.palette_ram.get(index) {
                    Some(&value) => data = value,
                    None => {
                        error!("Couldn't read addr {:x?} from BG / OBJ Palette RAM", addr)
                    }
                }
            }

            0x06000000..=0x06FFFFFF => {
                let mirrored = (addr - 0x06000000) % (128 * 1024);
                let index = if mirrored >= 96 * 1024 {
                    (mirrored - 32 * 1024) as usize
                } else {
                    mirrored as usize
                };

                match self.vram.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couln't read addr {:x?} from VRAM", addr),
                }
            }

            0x07000000..=0x07FFFFFF => {
                let index = ((addr - 0x07000000) % self.oam.len() as u32) as usize;
                match self.oam.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from OAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from display memory", addr)
            }
        }
        data
    }

    fn read16(&self, addr: u32) -> u16 {
        let first = self.read8(addr) as u16;
        let second = self.read8(addr + 1) as u16;

        // let CLOCK
        //match addr {
        //    // bg obj palette ram
        //    0x05000000..=0x05FFFFFF => {
        //    }
        //
        //    // vram
        //    0x06000000..=0x06FFFFFF => {
        //    }
        //
        //    // oam
        //    0x07000000..=0x07FFFFFF => {
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from display memory", addr)
        //    }
        //}

        first | (second << 8)
    }

    fn read32(&self, addr: u32) -> u32 {
        let first = self.read16(addr) as u32;
        let second = self.read16(addr + 2) as u32;

        // let CLOCK
        //match addr {
        //    // bg obj palette ram
        //    0x05000000..=0x05FFFFFF => {
        //    }
        //
        //    // vram
        //    0x06000000..=0x06FFFFFF => {
        //    }
        //
        //    // oam
        //    0x07000000..=0x07FFFFFF => {
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from display memory", addr)
        //    }
        //}

        first | (second << 16)
    }
}

impl ExternalMemory {
    fn read8(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                let index = ((addr - 0x08000000) % self.rom.len() as u32) as usize;
                match self.rom.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from GamePak ROM", addr),
                }
            }

            0x0E000000..=0x0FFFFFFF => {
                let index = ((addr - 0x0E000000) % self.sram.len() as u32) as usize;
                match self.sram.get(index) {
                    Some(&value) => data = value,
                    None => error!("Couldn't read addr {:x?} from GamePak SRAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from external memory", addr)
            }
        }

        data
    }

    fn read16(&self, addr: u32) -> u16 {
        let first = self.read8(addr) as u16;
        let second = self.read8(addr + 1) as u16;

        // let clk
        //match addr {
        //    0x08000000..=0x0DFFFFFF => {
        //       // let wait: usize = addr as usize / self.external.rom.len();
        //    }
        //
        //    0x0E000000..=0x0FFFFFFF => {
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from external memory", addr)
        //    }
        //}

        first | (second << 8)
    }

    fn read32(&self, addr: u32) -> u32 {
        let first = self.read16(addr) as u32;
        let second = self.read16(addr + 2) as u32;

        // let clk
        //match addr {
        //    0x08000000..=0x0DFFFFFF => {
        //       // let wait: usize = addr as usize / self.external.rom.len();
        //    }
        //
        //    0x0E000000..=0x0FFFFFFF => {
        //    }
        //
        //    _ => {
        //        error!("Couldn't read addr {:x?} from external memory", addr)
        //    }
        //}

        first | (second << 16)
    }
}

impl GBAMemory {
    // TODO: UNUSED MEMORY READ/WRITE SPECIAL CASE FUNCTION

    // TODO: IMPLEMENT CLOCK TIME TRACKING
    pub fn read8(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;
        // let mut clk: u8 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.internal.read8(addr),

            0x05000000..=0x07FFFFFF => data = self.display.read8(addr),

            0x08000000..=0x0FFFFFFF => data = self.external.read8(addr),

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }

    pub fn read16(&self, addr: u32) -> u16 {
        let mut data: u16 = 0;
        // let mut clk: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.internal.read16(addr),

            0x05000000..=0x07FFFFFF => data = self.display.read16(addr),

            0x08000000..=0x0FFFFFFF => data = self.external.read16(addr),

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }

    pub fn read32(&self, addr: u32) -> u32 {
        let mut data: u32 = 0;
        // let mut clk: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.internal.read32(addr),

            0x05000000..=0x07FFFFFF => data = self.display.read32(addr),

            0x08000000..=0x0FFFFFFF => data = self.external.read32(addr),

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }
}

// MEMORY WRITES
