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

    // TODO: UNUSED MEMORY READ/WRITE SPECIAL CASE FUNCTION

    pub fn read8(&mut self, addr: u32) -> u8 {
        let mut result: u8 = 0;

        match addr {
            // Internal Memory
            0x00000000..=0x04FFFFFF => match addr {
                // EDGECASE
                0x00000000..=0x00003FFF => {
                    match self.internal.bios.get((addr - 0x00000000) as usize) {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from BIOS", addr),
                    }
                }

                0x02000000..=0x02FFFFFF => {
                    match self.internal.wram_on_board.get(
                        ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32) as usize,
                    ) {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from On-Board WRAM", addr),
                    }
                }

                0x03000000..=0x03FFFFFF => {
                    match self.internal.wram_on_chip.get(
                        ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32) as usize,
                    ) {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from On-Chip WRAM", addr),
                    }
                }

                // EDGECASE
                0x04000000..=0x040003FE => {
                    match self.internal.io_registers.get((addr - 0x04000000) as usize) {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from I/O Registers", addr),
                    }
                }

                // Unused Memory Areas
                0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                    warn!("Accessing unused memory addr: {:x?}", addr);
                    // TODO: IMPLEMENT SPECIAL CASE
                }

                _ => {
                    result = 0;
                    error!("Couldn't read addr {:x?} from internal memory", addr)
                }
            },

            // Display Memory
            0x05000000..=0x07FFFFFF => {}

            // External Memory
            0x08000000..=0x0FFFFFFF => {}

            _ => {}
        }

        result
    }

    pub fn read16(addr: u32) -> u16 {}

    pub fn read32(addr: u32) -> u16 {}
}
