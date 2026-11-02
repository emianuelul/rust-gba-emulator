use crate::register;

// 0 - pressed
// 1 - released

pub const KEYINPUT_ADDR: u32 = 0x0400_0130;
register!(
    RegKeyinput: u16 {
        a_btn: bool => (0, 1),
        b_btn: bool => (1, 1),
        select_btn: bool => (2, 1),
        start_btn: bool => (3, 1),
        right_btn: bool => (4, 1),
        left_btn: bool => (5, 1),
        up_btn: bool => (6, 1),
        down_btn: bool => (7, 1),
        r_btn: bool => (8, 1),
        l_btn: bool => (9, 1)
    }
);

pub const KEYCNT_ADDR: u32 = 0x0400_0132;
register!(
    RegKeycnt: u16 {
        a_btn: bool => (0, 1),
        b_btn: bool => (1, 1),
        select_btn: bool => (2, 1),
        start_btn: bool => (3, 1),
        right_btn: bool => (4, 1),
        left_btn: bool => (5, 1),
        up_btn: bool => (6, 1),
        down_btn: bool => (7, 1),
        r_btn: bool => (8, 1),
        l_btn: bool => (9, 1),
        irq_enable: bool => (14, 1),
        irq_condiiton: bool => (15, 1)
    }
);
