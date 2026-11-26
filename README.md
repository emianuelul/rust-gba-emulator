![Rusty GBA](./assets/logo.png)

A Rust Emulator which is compiled through WASM to run on a React frontend. 
This project is my Bachelor's Final Project, done as a way to study emulation, CPU architecture (namely the ARM7TDMI) and various other things.

LLM's were used in this project for tasks such as generating unit / integration tests and for prototyping, like the basic frontend. All Rust code was written by me, by hand, the final UI will also be created by me, from scratch. 

As of Nov 27, 2026, I have implemented:
- Memory Bus Functionality:
    - 8/16/32-bit reads/writes
    - Cycle-accurate timings for each memory region

- CPU Functionality
    - Fetch / Decode / Execute Flow
    - All ARM Opcodes (except SWI) + Tests
    - All THUMB Opcodes (except SWI) + Tests
    - Cycle-accurate timings for each instruction
    - CPU Register operations

- Display
    - Display Mode 0
    - Display Mode 3
        - ![m3_demo.gba from LIBTONC](./assets/m3_demo_running.png)
        - ![hello.gba from Jsmolka's gba-tests](./assets/hello_demo_running.png)
    - Display Mode 4
    - Display Mode 5

- Input Support

- Frontend
    - Basic screen showing actual outputted frames from loaded `.gba` files

## How To Use (so far)
Simply load a ROM (`.gba` file)

## Roadmap
3. Interrupts 
4. DMA 
5. SWI / BIOS (HLE)
6. Display Mode 1, 2
7. VFX for Display
7. File Saving / Save States
8. Sound

## Legal note
I am not affiliated with Nintendo in any way, shape or form. I DO NOT distribute and/or ship ROMs or BIOS dumps, users have to come with those themselves. Always legally acquire GBA ROMs.
