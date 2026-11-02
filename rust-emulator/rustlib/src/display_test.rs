#[cfg(test)]
mod test {
    use crate::GBAEngine;

    #[test]
    fn display_mode_3() {
        //init_tracing();

        let test = "/Users/iemi/Downloads/bin/m3_demo.gba";
        let rom = std::fs::read(test).expect("Test not found");

        let mut gba = GBAEngine::new(rom, false);

        loop {
            gba.step();
        }
    }
}
