use wasm_bindgen::prelude::wasm_bindgen;

pub mod constants;
mod cpu;
pub mod display_registers;
mod display_test;
mod keypad_input_registers;
mod memory_area;
pub mod ppu_module;
pub mod register_macro;
pub mod system_ctrl_registers;
pub mod utility_fns;

use crate::cpu::Cpu;
use crate::keypad_input_registers::{RegKeycnt, RegKeyinput, KEYCNT_ADDR, KEYINPUT_ADDR};
use crate::memory_area::GBAMemory;
use crate::ppu_module::PPU;
use std::sync::Once;

static INIT: Once = Once::new();

fn init_keyinput(memory: &mut GBAMemory) {
    let mut keycnt: RegKeycnt = memory.read16(KEYCNT_ADDR).0.into();
    let mut keyinput: RegKeyinput = memory.read16(KEYINPUT_ADDR).0.into();

    keycnt.a_btn = true;
    keyinput.a_btn = true;

    keycnt.b_btn = true;
    keyinput.b_btn = true;

    keycnt.select_btn = true;
    keyinput.select_btn = true;

    keycnt.start_btn = true;
    keyinput.start_btn = true;

    keycnt.right_btn = true;
    keyinput.right_btn = true;

    keycnt.left_btn = true;
    keyinput.left_btn = true;

    keycnt.up_btn = true;
    keyinput.up_btn = true;

    keycnt.down_btn = true;
    keyinput.down_btn = true;

    keycnt.r_btn = true;
    keyinput.r_btn = true;

    keycnt.l_btn = true;
    keyinput.l_btn = true;

    memory.write16(KEYCNT_ADDR, keycnt.into());
    memory.write16(KEYINPUT_ADDR, keyinput.into());
}

#[wasm_bindgen]
pub struct GBAEngine {
    memory: GBAMemory,
    cpu: Cpu,
    ppu: PPU,
    pending_clk: u32,
}

#[wasm_bindgen]
impl GBAEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(rom_data: Vec<u8>, debug: bool) -> Self {
        let mut engine = GBAEngine {
            memory: GBAMemory::new(rom_data),
            cpu: Cpu::new(),
            ppu: PPU::new(),
            pending_clk: 0,
        };

        if debug {
            INIT.call_once(|| {
                tracing_wasm::set_as_global_default();
                std::panic::set_hook(Box::new(|info| {
                    tracing::error!("{info}");
                }));
            });
        }

        // init first 2 fetches
        engine.pending_clk = engine
            .cpu
            .get_refill_clk(&engine.memory, engine.cpu.registers.pc);

        init_keyinput(&mut engine.memory);

        engine
    }
}

#[wasm_bindgen]
impl GBAEngine {
    pub fn get_screen_pointer(&self) -> *const u32 {
        self.ppu.display.screen.as_ptr()
    }

    pub fn update_key(&mut self, key: u8, released: bool) {
        let mut keycnt: RegKeycnt = self.memory.read16(KEYCNT_ADDR).0.into();
        let mut keyinput: RegKeyinput = self.memory.read16(KEYINPUT_ADDR).0.into();

        match key {
            0 => {
                keycnt.a_btn = released;
                keyinput.a_btn = released;
            }
            1 => {
                keycnt.b_btn = released;
                keyinput.b_btn = released;
            }
            2 => {
                keycnt.select_btn = released;
                keyinput.select_btn = released;
            }
            3 => {
                keycnt.start_btn = released;
                keyinput.start_btn = released;
            }
            4 => {
                keycnt.right_btn = released;
                keyinput.right_btn = released;
            }
            5 => {
                keycnt.left_btn = released;
                keyinput.left_btn = released;
            }
            6 => {
                keycnt.up_btn = released;
                keyinput.up_btn = released;
            }
            7 => {
                keycnt.down_btn = released;
                keyinput.down_btn = released;
            }
            8 => {
                keycnt.r_btn = released;
                keyinput.r_btn = released;
            }
            9 => {
                keycnt.l_btn = released;
                keyinput.l_btn = released;
            }
            _ => {
                unreachable!()
            }
        }

        self.memory.write16(KEYCNT_ADDR, keycnt.into());
        self.memory.write16(KEYINPUT_ADDR, keyinput.into());
    }
}

#[wasm_bindgen]
impl GBAEngine {
    pub fn step(&mut self) -> bool {
        let mut draw = false;

        let clk = std::mem::take(&mut self.pending_clk) + self.cpu.step(&mut self.memory);
        if self.ppu.step(&mut self.memory, clk) {
            draw = true;
        }

        draw
    }
}
