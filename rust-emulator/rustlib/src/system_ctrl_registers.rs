use crate::{display_registers::get_register32_field_value, memory_area::GBAMemory};

const TEMP_WAITCNT: u32 = 0x4317;
pub const WAITCNT_ADDR: u32 = 0x04000204;

// first - N
// second - S
pub const WAITCNT_SRAM_WAIT: (u8, u8) = (0, 2);
pub const WAITCNT_WS0_FIRST: (u8, u8) = (2, 2);
pub const WAITCNT_WS0_SECOND: (u8, u8) = (4, 1);
pub const WAITCNT_WS1_FIRST: (u8, u8) = (5, 2);
pub const WAITCNT_WS1_SECOND: (u8, u8) = (7, 1);
pub const WAITCNT_WS2_FIRST: (u8, u8) = (8, 2);
pub const WAITCNT_WS2_SECOND: (u8, u8) = (10, 1);
pub const WAITCNT_PHI: (u8, u8) = (11, 2);
pub const WAITCNT_PREFETCH: (u8, u8) = (14, 2);
pub const WAITCNT_GAMEPAK_TYPE: (u8, u8) = (15, 1);

// 32bit
pub struct WAITCNT {
    sram_wait: u8,
    ws0_first: u8,
    ws0_second: bool,
    ws1_first: u8,
    ws1_second: bool,
    ws2_first: u8,
    ws2_second: bool,
    phi: u8,
    prefetch: bool,
    gamepak_type: bool,
}

impl GBAMemory {
    pub fn get_waitcnt(&self) -> WAITCNT {
        // let register = self.read32(WAITCNT_ADDR).0;
        let register: u32 = TEMP_WAITCNT;

        let sram_wait = get_register32_field_value(register, WAITCNT_SRAM_WAIT) as u8;
        let ws0_first = get_register32_field_value(register, WAITCNT_WS0_FIRST) as u8;
        let ws0_second = get_register32_field_value(register, WAITCNT_WS0_SECOND) == 1;
        let ws1_first = get_register32_field_value(register, WAITCNT_WS1_FIRST) as u8;
        let ws1_second = get_register32_field_value(register, WAITCNT_WS1_SECOND) == 1;
        let ws2_first = get_register32_field_value(register, WAITCNT_WS2_SECOND) as u8;
        let ws2_second = get_register32_field_value(register, WAITCNT_WS2_SECOND) == 1;
        let phi = get_register32_field_value(register, WAITCNT_PHI) as u8;
        let prefetch = get_register32_field_value(register, WAITCNT_PREFETCH) == 1;
        let gamepak_type = get_register32_field_value(register, WAITCNT_GAMEPAK_TYPE) == 1;

        WAITCNT {
            sram_wait,
            ws0_first,
            ws0_second,
            ws1_first,
            ws1_second,
            ws2_first,
            ws2_second,
            phi,
            prefetch,
            gamepak_type,
        }
    }
}
