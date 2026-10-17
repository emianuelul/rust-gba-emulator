use super::GBAMemory;
use crate::utility_fns::{align16, align32, is_imc};
use tracing::{error, warn};

// INTERNAL MEMORY READS
impl GBAMemory {
    fn read8_internal(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                let index = addr as usize;
                match self.internal.bios.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from BIOS", addr),
                }
            }

            0x02000000..=0x02FFFFFF => {
                if self.cache.use_ewram {
                    let index =
                        ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32) as usize;
                    match self.internal.wram_on_board.get(index) {
                        Some(&value) => {
                            data = value;
                        }
                        None => error!("Couldn't read addr {:x?} from On-Board WRAM", addr),
                    }
                } else {
                    let index =
                        ((addr - 0x02000000) % self.internal.wram_on_chip.len() as u32) as usize;

                    match self.internal.wram_on_chip.get(index) {
                        Some(&value) => {
                            data = value;
                        }
                        None => error!("Couldn't read addr {:x?} from MIRRORED OnChip WRAM", addr),
                    }
                }
            }

            0x03000000..=0x03FFFFFF => {
                let index =
                    ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32) as usize;
                match self.internal.wram_on_chip.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from On-Chip WRAM", addr),
                }
            }

            // EDGECASE
            0x04000000..=0x040003FF => {
                let index = (addr - 0x04000000) as usize;
                match self.internal.io_registers.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from I/O Registers", addr),
                }
            }

            x if is_imc(x) => {
                let index = (addr & 0x3) as usize;
                match self.internal.imc.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read byte for IMC"),
                }
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                let raw_data = self.fetched_opcode.get();
                data = (raw_data >> ((addr & 3) * 8)) as u8;
            }

            _ => {
                error!("Couldn't read addr {:x?} from internal memory", addr)
            }
        }

        data
    }

    fn read16_internal(&self, addr: u32) -> u16 {
        let first = self.read8_internal(addr) as u16;
        let second = self.read8_internal(addr + 1) as u16;

        first | (second << 8)
    }

    fn read32_internal(&self, addr: u32) -> u32 {
        let first = self.read16_internal(addr) as u32;
        let second = self.read16_internal(addr + 2) as u32;

        first | (second << 16)
    }
}

// TODO: revisit after ppu (+1 clk if memory is accessed during rendering)
// DISPLAY MEMORY READS
impl GBAMemory {
    fn read8_display(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;

        match addr {
            0x05000000..=0x05FFFFFF => {
                let index = ((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize;
                match self.display.palette_ram.get(index) {
                    Some(&value) => {
                        data = value;
                    }
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

                match self.display.vram.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couln't read addr {:x?} from VRAM", addr),
                }
            }

            0x07000000..=0x07FFFFFF => {
                let index = ((addr - 0x07000000) % self.display.oam.len() as u32) as usize;
                match self.display.oam.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from OAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from display memory", addr)
            }
        }

        data
    }

    fn read16_display(&self, addr: u32) -> u16 {
        let first = self.read8_display(addr) as u16;
        let second = self.read8_display(addr + 1) as u16;

        first | (second << 8)
    }

    fn read32_display(&self, addr: u32) -> u32 {
        let first = self.read16_display(addr) as u32;
        let second = self.read16_display(addr + 2) as u32;

        first | (second << 16)
    }
}

// TODO: revisit rom memory after waitcnt
// EXTERNAL MEMORY READS
impl GBAMemory {
    fn read8_external(&self, addr: u32) -> u8 {
        let mut data: u8 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                let offset = (addr & 0x01FFFFFF) as usize;
                data = match self.external.rom.get(offset) {
                    Some(&value) => value,
                    None => (((addr >> 1) & 0xFFFF) >> ((addr & 1) * 8)) as u8,
                };
            }

            0x0E000000..=0x0FFFFFFF => {
                let index = ((addr - 0x0E000000) % self.external.sram.len() as u32) as usize;
                match self.external.sram.get(index) {
                    Some(&value) => {
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from GamePak SRAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from external memory", addr)
            }
        }

        data
    }

    fn read16_external(&self, addr: u32) -> u16 {
        let first = self.read8_external(addr) as u16;
        let second = self.read8_external(addr + 1) as u16;

        first | (second << 8)
    }

    fn read32_external(&self, addr: u32) -> u32 {
        let first = self.read16_external(addr) as u32;
        let second = self.read16_external(addr + 2) as u32;

        first | (second << 16)
    }
}

// GBAMEMORY READS
impl GBAMemory {
    // TODO: UNUSED MEMORY READ/WRITE SPECIAL CASE FUNCTION
    pub fn read8(&self, addr: u32) -> (u8, u32) {
        let mut data: u8 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read8_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read8_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read8_external(addr);
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        let clk: u32 = self.get_access_clk(addr, 1, self.is_sequential(addr));
        self.last_access.set((addr, 1));
        (data, clk)
    }

    pub fn read16(&self, addr: u32) -> (u16, u32) {
        let addr = align16(addr);

        let mut data: u16 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read16_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read16_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read16_external(addr);
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        let clk: u32 = self.get_access_clk(addr, 2, self.is_sequential(addr));
        self.last_access.set((addr, 2));
        (data, clk)
    }

    pub fn peek16(&self, addr: u32) -> u16 {
        let addr = align16(addr);
        match addr {
            0x00000000..=0x04FFFFFF => self.read16_internal(addr),
            0x05000000..=0x07FFFFFF => self.read16_display(addr),
            0x08000000..=0x0FFFFFFF => self.read16_external(addr),
            _ => 0,
        }
    }

    pub fn read32(&self, addr: u32) -> (u32, u32) {
        let addr = align32(addr);

        let mut data: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read32_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read32_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read32_external(addr);
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        let clk: u32 = self.get_access_clk(addr, 4, self.is_sequential(addr));
        self.last_access.set((addr, 4));
        (data, clk)
    }

    pub fn peek32(&self, addr: u32) -> u32 {
        let addr = align32(addr);
        match addr {
            0x00000000..=0x04FFFFFF => self.read32_internal(addr),
            0x05000000..=0x07FFFFFF => self.read32_display(addr),
            0x08000000..=0x0FFFFFFF => self.read32_external(addr),
            _ => 0,
        }
    }
}
