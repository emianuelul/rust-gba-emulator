use std::cell::Cell;
use tracing::{error, warn};

use crate::system_ctrl_registers::WAITCNT_ADDR;

fn touches_waitcnt(addr: u32, width: u8) -> bool {
    addr < WAITCNT_ADDR + 2 && WAITCNT_ADDR < addr + width as u32
}

fn get_rom_mirror_clk(addr: u32, mem: &GBAMemory) -> u32 {
    let wait = (addr >> 25) & 3;
    match wait {
        0 => mem.ns0_wait.0 + mem.ns0_wait.1,

        1 => mem.ns1_wait.0 + mem.ns1_wait.1,

        2 => mem.ns2_wait.0 + mem.ns2_wait.1,

        _ => {
            unreachable!()
        }
    }
}

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
    rom: Vec<u8>, // wait0 32Mb      0x08000000 - 0x09FFFFFF mirrored wait1 32Mb           0x0A000000 - 0x0BFFFFFF
    // mirrored wait2 32Mb           0x0C000000 - 0x0DFFFFFF
    sram: Vec<u8>, // 64Kb           0x0E000000 - 0x0E00FFFF
                   // unused ~32Mb   0x0E010000 - 0x0FFFFFFF
}

pub struct GBAMemory {
    internal: InternalMemory,
    display: DisplayMemory,
    external: ExternalMemory,
    // TODO: use once n/s logic exists
    last_access: Cell<u32>,
    // unused 0x10000000 - 0xFFFFFFFF
    pub ns0_wait: (u32, u32),
    pub ns1_wait: (u32, u32),
    pub ns2_wait: (u32, u32),
    pub ns_sram_wait: u32,
}

// INIT
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

// TEMPORARY WAITCNT REMOVE AFTER BIOS
const TEMP_WAITCNT: u16 = 0x4317;
impl GBAMemory {
    pub fn new(rom_data: Vec<u8>) -> Self {
        let mut returned = GBAMemory {
            internal: InternalMemory::new(),
            display: DisplayMemory::new(),
            external: ExternalMemory::new(rom_data),
            last_access: 0.into(),
            ns0_wait: (0, 0),
            ns1_wait: (0, 0),
            ns2_wait: (0, 0),
            ns_sram_wait: 0,
        };

        // REMOVE: AFTER BIOS INIT
        returned.write16(WAITCNT_ADDR, TEMP_WAITCNT);

        returned
    }

    pub fn get_rom_size(&self) -> usize {
        self.external.rom.len()
    }

    pub fn get_rom_end_addr(&self) -> usize {
        0x08000000 + self.external.rom.len()
    }
}

fn align16(addr: u32) -> u32 {
    addr & !1u32
}

fn align32(addr: u32) -> u32 {
    addr & !3u32
}

// INTERNAL MEMORY READS
impl GBAMemory {
    fn read8_internal(&self, addr: u32) -> (u8, u32) {
        let mut data: u8 = 0;
        let mut clk: u32 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
                let index = addr as usize;
                match self.internal.bios.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = 1;
                    }
                    None => error!("Couldn't read addr {:x?} from BIOS", addr),
                }
            }

            0x02000000..=0x02FFFFFF => {
                let index =
                    ((addr - 0x02000000) % self.internal.wram_on_board.len() as u32) as usize;
                match self.internal.wram_on_board.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = 3;
                    }
                    None => error!("Couldn't read addr {:x?} from On-Board WRAM", addr),
                }
            }

            0x03000000..=0x03FFFFFF => {
                let index =
                    ((addr - 0x03000000) % self.internal.wram_on_chip.len() as u32) as usize;
                match self.internal.wram_on_chip.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = 1;
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
                        clk = 1;
                    }
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

        (data, clk)
    }

    fn read16_internal(&self, addr: u32) -> (u16, u32) {
        let first = self.read8_internal(addr).0 as u16;
        let second = self.read8_internal(addr + 1).0 as u16;

        let mut clk: u32 = 0;
        match addr {
            // bios
            0x00000000..=0x00003FFF => {
                clk = 1;
            }

            // wram onboard
            0x02000000..=0x02FFFFFF => {
                clk = 3;
                // TODO: revisit after waitcnt
            }

            // wram onchip
            0x03000000..=0x03FFFFFF => {
                clk = 1;
            }

            // io registers
            0x04000000..=0x040003FF => {
                clk = 1;
            }

            // Unused Mem Areas
            0x00004000..=0x01FFFFFF | 0x04000400..=0x04FFFFFF => {
                warn!("Accessing unused memory addr: {:x?}", addr);
                // TODO: unused memory special case
            }

            _ => {
                error!("Couldn't read addr {:x?} from internal memory", addr)
            }
        }

        (first | (second << 8), clk)
    }

    fn read32_internal(&self, addr: u32) -> (u32, u32) {
        let first = self.read16_internal(addr).0 as u32;
        let second = self.read16_internal(addr + 2).0 as u32;

        let mut clk: u32 = 0;
        match addr {
            // EDGECASE
            0x00000000..=0x00003FFF => {
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
                // TODO: unused memory special case
            }

            _ => {
                error!("Couldn't read addr {:x?} from internal memory", addr)
            }
        }

        (first | (second << 16), clk)
    }
}

// TODO: revisit after ppu (+1 clk if memory is accessed during rendering)
// DISPLAY MEMORY READS
impl GBAMemory {
    fn read8_display(&self, addr: u32) -> (u8, u32) {
        let mut data: u8 = 0;
        let mut clk: u32 = 0;

        match addr {
            0x05000000..=0x05FFFFFF => {
                let index = ((addr - 0x05000000) % self.display.palette_ram.len() as u32) as usize;
                match self.display.palette_ram.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = 1;
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
                        clk = 1;
                    }
                    None => error!("Couln't read addr {:x?} from VRAM", addr),
                }
            }

            0x07000000..=0x07FFFFFF => {
                let index = ((addr - 0x07000000) % self.display.oam.len() as u32) as usize;
                match self.display.oam.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = 1;
                    }
                    None => error!("Couldn't read addr {:x?} from OAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from display memory", addr)
            }
        }
        (data, clk)
    }

    fn read16_display(&self, addr: u32) -> (u16, u32) {
        let first = self.read8_display(addr).0 as u16;
        let second = self.read8_display(addr + 1).0 as u16;

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
                error!("Couldn't read addr {:x?} from display memory", addr)
            }
        }

        (first | (second << 8), clk)
    }

    fn read32_display(&self, addr: u32) -> (u32, u32) {
        let first = self.read16_display(addr).0 as u32;
        let second = self.read16_display(addr + 2).0 as u32;

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
                error!("Couldn't read addr {:x?} from display memory", addr)
            }
        }

        (first | (second << 16), clk)
    }
}

const ROM_MIRROR_TEMP_CLK: u8 = 67;
const SRAM_TEMP_CLK: u8 = 69;

// TODO: revisit rom memory after waitcnt
// EXTERNAL MEMORY READS
impl GBAMemory {
    fn read8_external(&self, addr: u32) -> (u8, u32) {
        let mut data: u8 = 0;
        let mut clk: u32 = 0;

        match addr {
            0x08000000..=0x0DFFFFFF => {
                let index = ((addr - 0x08000000) % self.external.rom.len() as u32) as usize;
                match self.external.rom.get(index) {
                    Some(&value) => {
                        clk = ROM_MIRROR_TEMP_CLK as u32;
                        data = value;
                    }
                    None => error!("Couldn't read addr {:x?} from GamePak ROM", addr),
                }
            }

            0x0E000000..=0x0FFFFFFF => {
                let index = ((addr - 0x0E000000) % self.external.sram.len() as u32) as usize;
                match self.external.sram.get(index) {
                    Some(&value) => {
                        data = value;
                        clk = SRAM_TEMP_CLK as u32;
                    }
                    None => error!("Couldn't read addr {:x?} from GamePak SRAM", addr),
                }
            }

            _ => {
                error!("Couldn't read addr {:x?} from external memory", addr)
            }
        }

        (data, clk)
    }

    fn read16_external(&self, addr: u32) -> (u16, u32) {
        let first = self.read8_external(addr).0 as u16;
        let second = self.read8_external(addr + 1).0 as u16;

        let mut clk: u32 = 0;
        match addr {
            0x08000000..=0x0DFFFFFF => {
                clk = ROM_MIRROR_TEMP_CLK as u32;
            }

            0x0E000000..=0x0FFFFFFF => {
                warn!("Reading 16bit value from FLASH memory");
                clk = SRAM_TEMP_CLK as u32;
            }

            _ => {
                error!("Couldn't read addr {:x?} from external memory", addr)
            }
        }

        (first | (second << 8), clk)
    }

    fn read32_external(&self, addr: u32) -> (u32, u32) {
        let first = self.read16_external(addr).0 as u32;
        let second = self.read16_external(addr + 2).0 as u32;

        let mut clk: u32 = 0;
        match addr {
            0x08000000..=0x0DFFFFFF => {
                clk = ROM_MIRROR_TEMP_CLK as u32;
            }

            0x0E000000..=0x0FFFFFFF => {
                warn!("Reading 32bit value from FLASH memory");
                clk = SRAM_TEMP_CLK as u32;
            }

            _ => {
                error!("Couldn't read addr {:x?} from external memory", addr)
            }
        }

        (first | (second << 16), clk)
    }
}

// GBAMEMORY READS
impl GBAMemory {
    // TODO: UNUSED MEMORY READ/WRITE SPECIAL CASE FUNCTION
    pub fn read8(&self, addr: u32) -> (u8, u32) {
        let mut data: (u8, u32) = (0, 0);

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read8_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read8_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read8_external(addr);
                if data.1 == ROM_MIRROR_TEMP_CLK as u32 {
                    data.1 = get_rom_mirror_clk(addr, self);
                } else if data.1 == SRAM_TEMP_CLK as u32 {
                    data.1 = self.ns_sram_wait;
                }
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }

    pub fn read16(&self, addr: u32) -> (u16, u32) {
        let addr = align16(addr);

        let mut data: (u16, u32) = (0, 0);

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read16_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read16_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read16_external(addr);
                if data.1 == ROM_MIRROR_TEMP_CLK as u32 {
                    data.1 = get_rom_mirror_clk(addr, self);
                } else if data.1 == SRAM_TEMP_CLK as u32 {
                    data.1 = self.ns_sram_wait;
                }
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }

    pub fn read32(&self, addr: u32) -> (u32, u32) {
        let addr = align32(addr);

        let mut data: (u32, u32) = (0, 0);

        match addr {
            0x00000000..=0x04FFFFFF => data = self.read32_internal(addr),

            0x05000000..=0x07FFFFFF => data = self.read32_display(addr),

            0x08000000..=0x0FFFFFFF => {
                data = self.read32_external(addr);
                if data.1 == ROM_MIRROR_TEMP_CLK as u32 {
                    data.1 = get_rom_mirror_clk(addr, self);
                } else if data.1 == SRAM_TEMP_CLK as u32 {
                    data.1 = self.ns_sram_wait;
                }
            }

            _ => error!(
                "Couldn't read 8bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        data
    }
}

// MEMORY WRITES
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
                clk = ROM_MIRROR_TEMP_CLK as u32;
                warn!("Tried to write to ROM memory");
            }

            0x0E000000..=0x0FFFFFFF => {
                let index = ((addr - 0x0E000000) % self.external.sram.len() as u32) as usize;
                self.external.sram[index] = data;

                clk = SRAM_TEMP_CLK as u32;
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
                clk = ROM_MIRROR_TEMP_CLK as u32;
                warn!("Tried to write 16bit to ROM memory");
                return clk;
            }

            0x0E000000..=0x0FFFFFFF => {
                clk = SRAM_TEMP_CLK as u32;
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
                clk = ROM_MIRROR_TEMP_CLK as u32;
                warn!("Tried to write 32bit value to ROM");
                return clk;
            }

            0x0E000000..=0x0FFFFFFF => {
                clk = SRAM_TEMP_CLK as u32;
            }

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

                if clk == ROM_MIRROR_TEMP_CLK as u32 {
                    clk = get_rom_mirror_clk(addr, self);
                } else if clk == SRAM_TEMP_CLK as u32 {
                    clk = self.ns_sram_wait;
                }
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
                if clk == ROM_MIRROR_TEMP_CLK as u32 {
                    clk = get_rom_mirror_clk(addr, self);
                } else if clk == SRAM_TEMP_CLK as u32 {
                    clk = self.ns_sram_wait;
                }
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
                if clk == ROM_MIRROR_TEMP_CLK as u32 {
                    clk = get_rom_mirror_clk(addr, self);
                } else if clk == SRAM_TEMP_CLK as u32 {
                    clk = self.ns_sram_wait;
                }
            }

            _ => error!(
                "Couldn't write 32bit value from addr {:x?} from anywhere in memory",
                addr
            ),
        }

        clk
    }
}
