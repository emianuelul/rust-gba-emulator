use super::{get_rom_mirror_clk, GBAMemory};
use crate::utility_fns::{align16, align32, touches_waitcnt};
use tracing::{error, warn};

// TODO: revisit on board wram after waitcnt
// INTERNAL MEMORY WRITES
impl GBAMemory {
    fn write8_internal(&mut self, addr: u32, data: u8) -> u32 {
        let mut clk: u32 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                warn!(
                    "Attempted to write to BIOS which is Read-Only | {:x?} <- {:x?}",
                    addr, data
                );
                clk = 1;
                // let index = addr as usize;
                // self.bios[index] = data;
            }

            0x02000000..=0x02FFFFFF => {
                let index =
                    ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32) as usize;
                self.internal.wram_on_board[index] = data;
                clk = 3;
            }

            0x03000000..=0x03FFFFFF => {
                let index =
                    ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32) as usize;
                self.internal.wram_on_chip[index] = data;
                clk = 1;
            }

            // EDGECASE
            0x04000000..=0x040003FF => {
                let index = (addr - 0x04000000) as usize;
                self.internal.io_registers[index] = data;
                clk = 1;
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                warn!("Accessing unused memory addr: {:x?}", addr);
                // TODO: IMPLEMENT SPECIAL CASE
            }

            _ => {
                error!("Couldn't write to addr {:x?} from internal memory", addr)
            }
        }
        clk
    }

    fn write16_internal(&mut self, addr: u32, data: u16) -> u32 {
        let mut clk: u32 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                warn!(
                    "Attempted to write to BIOS which is Read-Only | {:x?} <- {:x?}",
                    addr, data
                );
                // let index = addr as usize;
                // self.bios[index] = data;
                clk = 1;
            }

            0x02000000..=0x02FFFFFF => {
                clk = 3;
            }

            0x03000000..=0x03FFFFFF => {
                clk = 1;
            }

            // EDGECASE
            0x04000000..=0x040003FF => {
                clk = 1;
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                warn!("Accessing unused memory addr: {:x?}", addr);
                // TODO: IMPLEMENT SPECIAL CASE
            }

            _ => {
                error!("Couldn't write to addr {:x?} from internal memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        let _ = self.write8_internal(addr, bytes[0]);
        let _ = self.write8_internal(addr + 1, bytes[1]);

        clk
    }

    fn write32_internal(&mut self, addr: u32, data: u32) -> u32 {
        let mut clk: u32 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                warn!(
                    "Attempted to write to BIOS which is Read-Only | {:x?} <- {:x?}",
                    addr, data
                );
                // let index = addr as usize;
                // self.bios[index] = data;
                clk = 1;
            }

            0x02000000..=0x02FFFFFF => {
                clk = 6;
            }

            0x03000000..=0x03FFFFFF => {
                clk = 1;
            }

            // EDGECASE
            0x04000000..=0x040003FF => {
                clk = 1;
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                warn!("Accessing unused memory addr: {:x?}", addr);
                // TODO: IMPLEMENT SPECIAL CASE
            }

            _ => {
                error!("Couldn't write to addr {:x?} from internal memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        let _ = self.write8_internal(addr, bytes[0]);
        let _ = self.write8_internal(addr + 1, bytes[1]);
        let _ = self.write8_internal(addr + 2, bytes[2]);
        let _ = self.write8_internal(addr + 3, bytes[3]);

        clk
    }
}

// TODO: revisit after ppu (+1 clk if during scanline render)
// DISPLAY MEMORY WRITES
impl GBAMemory {
    fn write8_display(&mut self, addr: u32, data: u8) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            0x05000000..=0x05FFFFFF => {
                let index = ((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize;
                self.display.palette_ram[index] = data;

                clk = 1;
            }

            0x06000000..=0x06FFFFFF => {
                let mirrored = (addr - 0x06000000) % (128 * 1024);
                let index = if mirrored >= 96 * 1024 {
                    (mirrored - 32 * 1024) as usize
                } else {
                    mirrored as usize
                };

                self.display.vram[index] = data;

                clk = 1;
            }

            0x07000000..=0x07FFFFFF => {
                let index = ((addr - 0x07000000) % self.display.oam.len() as u32) as usize;
                self.display.oam[index] = data;

                clk = 1;
            }

            _ => {
                error!("Couldn't write addr {:x?} from display memory", addr)
            }
        }
        clk
    }

    fn write16_display(&mut self, addr: u32, data: u16) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            // bg obj palette ram
            0x05000000..=0x05FFFFFF => {
                clk = 1;
            }

            // vram
            0x06000000..=0x06FFFFFF => {
                clk = 1;
            }

            // oam
            0x07000000..=0x07FFFFFF => {
                clk = 1;
            }

            _ => {
                error!("Couldn't write addr {:x?} from display memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        let _ = self.write8_display(addr, bytes[0]);
        let _ = self.write8_display(addr + 1, bytes[1]);

        clk
    }

    fn write32_display(&mut self, addr: u32, data: u32) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            // bg obj palette ram
            0x05000000..=0x05FFFFFF => {
                clk = 2;
            }

            // vram
            0x06000000..=0x06FFFFFF => {
                clk = 2;
            }

            // oam
            0x07000000..=0x07FFFFFF => {
                clk = 1;
            }

            _ => {
                error!("Couldn't write addr {:x?} from display memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        let _ = self.write8_display(addr, bytes[0]);
        let _ = self.write8_display(addr + 1, bytes[1]);
        let _ = self.write8_display(addr + 2, bytes[2]);
        let _ = self.write8_display(addr + 3, bytes[3]);
        clk
    }
}

// TODO: handle FLASH vs ROM memory writes
// EXTERNAL MEMORY WRITES
impl GBAMemory {
    fn write8_external(&mut self, addr: u32, data: u8) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                clk = get_rom_mirror_clk(addr, self);
                warn!("Tried to write to ROM memory");
            }

            0x0E000000..=0x0FFFFFFF => {
                let index = ((addr - 0x0E000000) % self.external.sram.len() as u32) as usize;
                self.external.sram[index] = data;

                clk = self.ns_sram_wait;
            }

            _ => {
                error!("Couldn't write addr {:x?} from external memory", addr)
            }
        }

        clk
    }

    fn write16_external(&mut self, addr: u32, data: u16) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                clk = get_rom_mirror_clk(addr, self);
                warn!("Tried to write 16bit to ROM memory");
                return clk;
            }

            0x0E000000..=0x0FFFFFFF => {
                clk = self.ns_sram_wait;
            }

            _ => {
                error!("Couldn't write addr {:x?} from external memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        let _ = self.write8_external(addr, bytes[0]);
        let _ = self.write8_external(addr + 1, bytes[1]);

        clk
    }

    fn write32_external(&mut self, addr: u32, data: u32) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                clk = get_rom_mirror_clk(addr, self);
                warn!("Tried to write 32bit value to ROM");
                return clk;
            }

            0x0E000000..=0x0FFFFFFF => clk = self.ns_sram_wait,

            _ => {
                error!("Couldn't write addr {:x?} from external memory", addr)
            }
        }

        let bytes = data.to_le_bytes();
        self.write8_external(addr, bytes[0]);
        self.write8_external(addr + 1, bytes[1]);
        self.write8_external(addr + 2, bytes[2]);
        self.write8_external(addr + 3, bytes[3]);

        clk
    }
}

impl GBAMemory {
    pub fn write8(&mut self, addr: u32, data: u8) -> u32 {
        let mut clk: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => {
                clk = self.write8_internal(addr, data);
                if touches_waitcnt(addr, 1) {
                    tracing::debug!("WAITCNT UPDATED!");
                    self.update_waitcnt_cache();
                }
            }

            0x05000000..=0x07FFFFFF => clk = self.write8_display(addr, data),

            0x08000000..=0x0FFFFFFF => {
                clk = self.write8_external(addr, data);
            }

            _ => error!(
                "Couldn't write 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        clk
    }

    pub fn write16(&mut self, addr: u32, data: u16) -> u32 {
        let addr = align16(addr);

        let mut clk: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => {
                clk = self.write16_internal(addr, data);
                if touches_waitcnt(addr, 2) {
                    tracing::debug!("WAITCNT UPDATED!");
                    self.update_waitcnt_cache();
                }
            }

            0x05000000..=0x07FFFFFF => clk = self.write16_display(addr, data),

            0x08000000..=0x0FFFFFFF => {
                clk = self.write16_external(addr, data);
            }

            _ => error!(
                "Couldn't write 16bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        clk
    }

    pub fn write32(&mut self, addr: u32, data: u32) -> u32 {
        let addr = align32(addr);

        let mut clk: u32 = 0;

        match addr {
            0x00000000..=0x04FFFFFF => {
                clk = self.write32_internal(addr, data);
                if touches_waitcnt(addr, 4) {
                    tracing::debug!("WAITCNT UPDATED!");
                    self.update_waitcnt_cache();
                }
            }

            0x05000000..=0x07FFFFFF => clk = self.write32_display(addr, data),

            0x08000000..=0x0FFFFFFF => {
                clk = self.write32_external(addr, data);
            }

            _ => error!(
                "Couldn't write 32bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        clk
    }
}
