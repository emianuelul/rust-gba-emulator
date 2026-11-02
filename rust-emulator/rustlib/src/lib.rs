use wasm_bindgen::prelude::wasm_bindgen;

pub mod constants;
mod cpu;
pub mod display_registers;
mod display_test;
mod memory_area;
pub mod ppu_module;
pub mod register_macro;
pub mod system_ctrl_registers;
pub mod utility_fns;

use crate::cpu::Cpu;
use crate::memory_area::GBAMemory;
use crate::ppu_module::PPU;
use std::sync::Once;

static INIT: Once = Once::new();

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

        engine
    }
}

#[wasm_bindgen]
impl GBAEngine {
    pub fn get_screen_pointer(&self) -> *const u32 {
        self.ppu.display.screen.as_ptr()
    }
}

#[wasm_bindgen]
impl GBAEngine {
    pub fn step(&mut self) -> bool {
        let mut draw = false;

        let clk = std::mem::take(&mut self.pending_clk) + self.cpu.step(&mut self.memory);
        if self.ppu.step(&mut self.memory, clk) {
            //println!("====== DRAWING FRAME ======");
            //println!("{:?}", self.ppu.get_frame());
            draw = true;
        }

        draw
    }
}
