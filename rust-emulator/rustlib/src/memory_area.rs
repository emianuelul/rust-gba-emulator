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

    // TODO: IMPLEMENT CLOCK TIME TRACKING
    pub fn read8(&mut self, addr: u32) -> u8 {
        let mut result: u8 = 0;
        // let mut clk: u8 = 0;

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

                // Unused Mem Areas
                0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                    warn!("Accessing unused memory addr: {:x?}", addr);
                    // TODO: IMPLEMENT SPECIAL CASE
                }

                _ => {
                    error!("Couldn't read addr {:x?} from internal memory", addr)
                }
            },

            // Display Memory
            0x05000000..=0x07FFFFFF => {
                match addr {
                    0x05000000..=0x05FFFFFF => {
                        match self.display.palette_ram.get(
                            ((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize,
                        ) {
                            Some(value) => result = *value,
                            None => {
                                error!("Couldn't read addr {:x?} from BG / OBJ Palette RAM", addr)
                            }
                        }
                    }

                    0x06000000..=0x06FFFFFF => {
                        let mirrored = (addr - 0x06000000) % (128 * 1024);
                        let final_offset = if mirrored >= 96 * 1024 {
                            mirrored - 32 * 1024
                        } else {
                            mirrored
                        };

                        match self.display.vram.get(final_offset as usize) {
                            Some(value) => result = *value,
                            None => error!("Couln't read addr {:x?} from VRAM", addr),
                        }
                    }

                    0x07000000..=0x07FFFFFF => {
                        match self
                            .display
                            .oam
                            .get(((addr - 0x07000000) % self.display.oam.len() as u32) as usize)
                        {
                            Some(value) => result = *value,
                            None => error!("Couldn't read addr {:x?} from OAM", addr),
                        }
                    }

                    _ => {
                        error!("Couldn't read addr {:x?} from display memory", addr)
                    }
                }
            }

            // External Memory
            0x08000000..=0x0FFFFFFF => match addr {
                0x08000000..=0x0DFFFFFF => {
                    // let wait: usize = addr as usize / self.external.rom.len();
                    match self
                        .external
                        .rom
                        .get(((addr - 0x08000000) % self.external.rom.len() as u32) as usize)
                    {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from GamePak ROM", addr),
                    }
                }

                0x0E000000..=0x0FFFFFFF => {
                    match self
                        .external
                        .sram
                        .get(((addr - 0x0E000000) % self.external.sram.len() as u32) as usize)
                    {
                        Some(value) => result = *value,
                        None => error!("Couldn't read addr {:x?} from GamePak SRAM", addr),
                    }
                }

                _ => {
                    error!("Couldn't read addr {:x?} from external memory", addr)
                }
            },

            _ => {
                error!("Couldn't read addr {:x?} from anywhere in memory", addr)
            }
        }

        result
    }

    pub fn read16(&mut self, addr: u32) -> u16 {
        let mut data: u16 = 0;

        match addr {
            // Internal Memory
            0x00000000..=0x04FFFFFF => match addr {
                // EDGECASE
                0x00000000..=0x00003FFF => {
                    match self.internal.bios.get((addr - 0x00000000) as usize) {
                        Some(&first) => {
                            match self.internal.bios.get((addr - 0x00000000) as usize + 1) {
                                Some(&second) => data = first as u16 | ((second as u16) << 8),

                                None => error!(
                                    "Couldn't read second byte for addr {:x?} from bios",
                                    addr
                                ),
                            }
                        }

                        None => error!("Couldn't read first byte for addr {:x?} from BIOS", addr),
                    }
                }

                0x02000000..=0x02FFFFFF => {
                    match self.internal.wram_on_board.get(
                        ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32) as usize,
                    ) {
                        Some(&first) => match self.internal.bios.get(
                            ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32)
                                as usize
                                + 1,
                        ) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!(
                                "Couldn't read second byte from addr {:x?} from On-Board WRAM",
                                addr
                            ),
                        },
                        None => error!(
                            "Couldn't read first byte from addr {:x?} from On-Board WRAM",
                            addr
                        ),
                    }
                }

                0x03000000..=0x03FFFFFF => {
                    match self.internal.wram_on_chip.get(
                        ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32) as usize,
                    ) {
                        Some(&first) => match self.internal.wram_on_chip.get(
                            ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32)
                                as usize
                                + 1,
                        ) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!("Couldn't read second byte from addr {:x?}", addr),
                        },
                        None => error!(
                            "Couldn't read first byte from addr {:x?} from On-Chip WRAM",
                            addr
                        ),
                    }
                }

                // EDGECASE
                0x04000000..=0x040003FE => {
                    match self.internal.io_registers.get((addr - 0x04000000) as usize) {
                        Some(&first) => match self
                            .internal
                            .io_registers
                            .get((addr - 0x04000000) as usize + 1)
                        {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!(
                                "Couldn't read second byte from addr {:x?} from I/O Registers",
                                addr
                            ),
                        },
                        None => error!(
                            "Couldn't read first byte from addr {:x?} from I/O Registers",
                            addr
                        ),
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
            },

            // Display Memory
            0x05000000..=0x07FFFFFF => match addr {
                0x05000000..=0x05FFFFFF => {
                    match self
                        .display
                        .palette_ram
                        .get(((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize)
                    {
                        Some(&first) => match self.display.palette_ram.get(
                            ((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize
                                + 1,
                        ) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!(
                                "Couldn't read second byte from addr {:x?} from BG / OBJ Palette RAM",
                                addr
                            ),
                        },
                        None => {
                            error!(
                                "Couldn't read first byte from addr {:x?} from BG / OBJ Palette RAM",
                                addr
                            )
                        }
                    }
                }

                0x06000000..=0x06FFFFFF => {
                    let mirrored = (addr - 0x06000000) % (128 * 1024);
                    let final_offset = if mirrored >= 96 * 1024 {
                        mirrored - 32 * 1024
                    } else {
                        mirrored
                    };

                    match self.display.vram.get(final_offset as usize) {
                        Some(&first) => match self.display.vram.get(final_offset as usize + 1) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => {
                                error!("Couldn't read second byte from addr {:x?} from VRAM", addr)
                            }
                        },
                        None => error!("Couln't read first byte from addr {:x?} from VRAM", addr),
                    }
                }

                0x07000000..=0x07FFFFFF => {
                    match self
                        .display
                        .oam
                        .get(((addr - 0x07000000) % self.display.oam.len() as u32) as usize)
                    {
                        Some(&first) => match self
                            .display
                            .oam
                            .get(((addr - 0x07000000) % self.display.oam.len() as u32) as usize)
                        {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => {
                                error!("Couldn't read second byte from addr {:x?} from OAM", addr)
                            }
                        },
                        None => error!("Couldn't read first byte addr {:x?} from OAM", addr),
                    }
                }

                _ => {
                    error!("Couldn't read addr {:x?} from display memory", addr)
                }
            },

            // External Memory
            0x08000000..=0x0FFFFFFF => match addr {
                0x08000000..=0x0DFFFFFF => {
                    // let wait: usize = addr as usize / self.external.rom.len();
                    match self
                        .external
                        .rom
                        .get(((addr - 0x08000000) % self.external.rom.len() as u32) as usize)
                    {
                        Some(&first) => match self.external.rom.get(
                            ((addr - 0x08000000) % self.external.rom.len() as u32) as usize + 1,
                        ) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!(
                                "Couldn't read second byte from addr {:x?} from GamePak ROM",
                                addr
                            ),
                        },
                        None => error!(
                            "Couldn't read first byte from addr {:x?} from GamePak ROM",
                            addr
                        ),
                    }
                }

                0x0E000000..=0x0FFFFFFF => {
                    match self
                        .external
                        .sram
                        .get(((addr - 0x0E000000) % self.external.sram.len() as u32) as usize)
                    {
                        Some(&first) => match self.external.sram.get(
                            ((addr - 0x0E000000) % self.external.sram.len() as u32) as usize + 1,
                        ) {
                            Some(&second) => data = first as u16 | ((second as u16) << 8),
                            None => error!(
                                "Couldn't read second byte from addr {:x?} from GamePak SRAM",
                                addr
                            ),
                        },
                        None => error!(
                            "Couldn't read first byte from addr {:x?} from GamePak SRAM",
                            addr
                        ),
                    }
                }

                _ => {
                    error!("Couldn't read addr {:x?} from external memory", addr)
                }
            },

            _ => {
                error!("Couldn't read addr {:x?} from anywhere in memory", addr)
            }
        }

        data
    }

    pub fn read32(addr: u32) -> u32 {}
}
