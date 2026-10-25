use crate::memory_area::GBAMemory;
use crate::register;

pub const WAITCNT_ADDR: u32 = 0x04000204;
const WAITCNT_SRAM_WAIT: (usize, usize) = (0, 2);
const WAITCNT_WS0_FIRST: (usize, usize) = (2, 2);
const WAITCNT_WS0_SECOND: (usize, usize) = (4, 1);
const WAITCNT_WS1_FIRST: (usize, usize) = (5, 2);
const WAITCNT_WS1_SECOND: (usize, usize) = (7, 1);
const WAITCNT_WS2_FIRST: (usize, usize) = (8, 2);
const WAITCNT_WS2_SECOND: (usize, usize) = (10, 1);
const WAITCNT_PHI: (usize, usize) = (11, 2);
const WAITCNT_PREFETCH: (usize, usize) = (14, 1);
const WAITCNT_GAMEPAK_TYPE: (usize, usize) = (15, 1);

register!(
    WAITCNT: u32 {
        sram_wait: u8 => WAITCNT_SRAM_WAIT,
        ws0_first: u8 => WAITCNT_WS0_FIRST,
        ws0_second: bool => WAITCNT_WS0_SECOND,
        ws1_first: u8 => WAITCNT_WS1_FIRST,
        ws1_second: bool => WAITCNT_WS1_SECOND,
        ws2_first: u8 => WAITCNT_WS2_FIRST,
        ws2_second: bool => WAITCNT_WS2_SECOND,
        phi: u8 => WAITCNT_PHI,
        prefetch: bool => WAITCNT_PREFETCH,
        gamepak_type: bool => WAITCNT_GAMEPAK_TYPE,
    }
);

pub const IMC_ADDR: u32 = 0x04000800;
const IMC_DISABLE_IE_WRAM: (usize, usize) = (0, 1);
const IMC_DISABLE_CGB_BOOTROM: (usize, usize) = (3, 1);
const IMC_ENABLE_WRAM: (usize, usize) = (5, 1);
const IMC_WAITCNT_EWRAM: (usize, usize) = (24, 4);

register! {
    InternalMemCnt: u32 {
        disable_ie_wram: bool => IMC_DISABLE_IE_WRAM,
        disable_cgb_bootrom: bool => IMC_DISABLE_CGB_BOOTROM,
        enable_ewram: bool => IMC_ENABLE_WRAM,
        waitcnt_ewram: u8 => IMC_WAITCNT_EWRAM,
    }
}

const N_WAIT_TIMES: [u32; 4] = [4, 3, 2, 8];

impl GBAMemory {
    pub fn update_imc_cache(&mut self) {
        let raw = self.read32(IMC_ADDR).0;
        let imc: InternalMemCnt = raw.into();

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
        let raw = self.read32(WAITCNT_ADDR).0;
        let waitcnt: WAITCNT = raw.into();

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
}
