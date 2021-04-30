use std::path::PathBuf;

#[cfg(feature = "debugger")]
use iced::{Application, Settings};
use nestadia_core::{Emulator, ExecutionMode};

mod rgb_value_table;
mod sdl_window;

#[cfg(feature = "debugger")]
mod debugger_window;

const NES_WIDTH: u32 = 256;
const NES_HEIGHT: u32 = 240;

pub(crate) struct EmulationState {
    emulator: Emulator,
    is_running: bool,
}

#[cfg(feature = "debugger")]
pub fn gui_start(rom: PathBuf) -> iced::Result {
    debugger_window::NestadiaIced::run(Settings {
        window: iced::window::Settings {
            min_size: Some((NES_WIDTH as u32, NES_HEIGHT as u32)),
            ..Default::default()
        },
        flags: debugger_window::NestadiaIcedRunFlags { rom_path: rom },
        ..Default::default()
    })
}

#[cfg(not(feature = "debugger"))]
pub fn gui_start(rom: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let rom = std::fs::read(rom).unwrap();

    let emulation_state = std::sync::Arc::new(std::sync::RwLock::new(EmulationState {
        emulator: Emulator::new(&rom, ExecutionMode::Ring3)?,
        is_running: true,
    }));

    sdl_window::start_game(emulation_state);
    Ok(())
}
