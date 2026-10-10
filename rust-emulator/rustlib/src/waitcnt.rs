use crate::display_registers::get_register_field_value;
use crate::memory_area::GBAMemory;

const TEMP_WAITCNT: usize = 0x4317;
pub const WAITCNT_ADDR: usize = 0x04000204;

// first - N
// second - S
pub enum WaitCntField {
    SramWait,
    Ws0First,
    Ws0Second,
    Ws1First,
    Ws1Second,
    Ws2First,
    Ws2Second,
    Phi,
    Prefetch,
    GamePakType,
}
pub fn get_waitcnt_field(field: WaitCntField) -> (u8, u8) {
    match field {
        WaitCntField::SramWait => (0, 2),
        WaitCntField::Ws0First => (2, 2),
        WaitCntField::Ws0Second => (4, 1),
        WaitCntField::Ws1First => (5, 2),
        WaitCntField::Ws1Second => (7, 1),
        WaitCntField::Ws2First => (8, 2),
        WaitCntField::Ws2Second => (10, 1),
        WaitCntField::Phi => (11, 2),
        WaitCntField::Prefetch => (14, 1),
        WaitCntField::GamePakType => (15, 1),
    }
}

// (N, S)
pub fn waitcnt_get_wait0_times(_memory: &mut GBAMemory) -> (u8, u8) {
    (3, 1)
}
