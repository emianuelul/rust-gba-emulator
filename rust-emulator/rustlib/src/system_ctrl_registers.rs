use crate::{
    display_registers::{get_register16_field_value, get_register32_field_value},
    memory_area::GBAMemory,
};

pub const WAITCNT_ADDR: u32 = 0x04000204;

// first - N
// second - S
const WAITCNT_SRAM_WAIT: (u8, u8) = (0, 2);
const WAITCNT_WS0_FIRST: (u8, u8) = (2, 2);
const WAITCNT_WS0_SECOND: (u8, u8) = (4, 1);
const WAITCNT_WS1_FIRST: (u8, u8) = (5, 2);
const WAITCNT_WS1_SECOND: (u8, u8) = (7, 1);
const WAITCNT_WS2_FIRST: (u8, u8) = (8, 2);
const WAITCNT_WS2_SECOND: (u8, u8) = (10, 1);
const WAITCNT_PHI: (u8, u8) = (11, 2);
const WAITCNT_PREFETCH: (u8, u8) = (14, 1);
const WAITCNT_GAMEPAK_TYPE: (u8, u8) = (15, 1);

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

const IMC_DISABLE_IE_WRAM: (u8, u8) = (0, 1);
const IMC_DISABLE_CGB_BOOTROM: (u8, u8) = (3, 1);
const IMC_ENABLE_WRAM: (u8, u8) = (5, 1);
const IMC_WAITCNT_EWRAM: (u8, u8) = (24, 4);

pub const IMC_ADDR: u32 = 0x04000800;
pub struct InternalMemCnt {
    disable_ie_wram: bool,
    disable_cgb_bootrom: bool,
    enable_ewram: bool,
    waitcnt_ewram: u8,
}

const N_WAIT_TIMES: [u32; 4] = [4, 3, 2, 8];

impl GBAMemory {
    pub fn update_imc_cache(&mut self) {
        let imc = self.get_imc();

        if imc.waitcnt_ewram == 15 {
            tracing::error!("EWRAM Wait = LOCKUP");
            return;
        }

        let ewram_wait = (15 - imc.waitcnt_ewram) + 1;
        let use_ewram = imc.enable_ewram;
        self.cache.ewram_wait = ewram_wait as u32;
        self.cache.use_ewram = use_ewram;
    }

    pub fn update_waitcnt_cache(&mut self) {
        let waitcnt = self.get_waitcnt();

        let n0 = N_WAIT_TIMES[waitcnt.ws0_first as usize];
        let s0 = if waitcnt.ws0_second {
            1
        } else {
            2
        };

        let n1 = N_WAIT_TIMES[waitcnt.ws1_first as usize];
        let s1 = if waitcnt.ws1_second {
            1
        } else {
            4
        };

        let n2 = N_WAIT_TIMES[waitcnt.ws2_first as usize];
        let s2 = if waitcnt.ws2_second {
            1
        } else {
            8
        };

        let sram_wait = N_WAIT_TIMES[waitcnt.sram_wait as usize];

        self.cache.ns0_wait = (n0 + 1, s0 + 1);
        self.cache.ns1_wait = (n1 + 1, s1 + 1);
        self.cache.ns2_wait = (n2 + 1, s2 + 1);
        self.cache.sram_wait = sram_wait + 1;
    }

    pub fn get_waitcnt(&self) -> WAITCNT {
        let register = self.read16(WAITCNT_ADDR).0;

        let sram_wait = get_register16_field_value(register, WAITCNT_SRAM_WAIT) as u8;
        let ws0_first = get_register16_field_value(register, WAITCNT_WS0_FIRST) as u8;
        let ws0_second = get_register16_field_value(register, WAITCNT_WS0_SECOND) == 1;
        let ws1_first = get_register16_field_value(register, WAITCNT_WS1_FIRST) as u8;
        let ws1_second = get_register16_field_value(register, WAITCNT_WS1_SECOND) == 1;
        let ws2_first = get_register16_field_value(register, WAITCNT_WS2_FIRST) as u8;
        let ws2_second = get_register16_field_value(register, WAITCNT_WS2_SECOND) == 1;
        let phi = get_register16_field_value(register, WAITCNT_PHI) as u8;
        let prefetch = get_register16_field_value(register, WAITCNT_PREFETCH) == 1;
        let gamepak_type = get_register16_field_value(register, WAITCNT_GAMEPAK_TYPE) == 1;

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

    pub fn get_imc(&self) -> InternalMemCnt {
        let register = self.read32(IMC_ADDR).0;

        let disable_ie_wram = get_register32_field_value(register, IMC_DISABLE_IE_WRAM) == 1;
        let disable_cgb_bootrom =
            get_register32_field_value(register, IMC_DISABLE_CGB_BOOTROM) == 1;
        let enable_ewram = get_register32_field_value(register, IMC_ENABLE_WRAM) == 1;
        let waitcnt_ewram = get_register32_field_value(register, IMC_WAITCNT_EWRAM) as u8;

        InternalMemCnt {
            disable_ie_wram,
            disable_cgb_bootrom,
            enable_ewram,
            waitcnt_ewram,
        }
    }
}
