#[cfg(test)]
mod test {
    use crate::gba_emulator::GBAEngine;

    const LOG: tracing::Level = tracing::Level::DEBUG;

    fn init_tracing() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(LOG)
            .try_init();
    }

    #[test]
    fn display_mode_3() {
        //init_tracing();

        let test = "/Users/iemi/Downloads/bin/m3_demo.gba";
        let rom = std::fs::read(test).expect("Test not found");

        let mut gba = GBAEngine::new(rom);

        loop {
            gba.step();
        }
    }
}
