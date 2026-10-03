const DISPCNT_ADDR: u32 = 0x04000000;

pub struct PPU {
    screen: [[u16; 160]; 260],
}

impl PPU {
    fn step(&mut self, clk: u32) {
        //
    }
}
