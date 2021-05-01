use std::path::PathBuf;

#[cfg(feature = "debugger")]
use iced::{Application, Settings};
use nestadia_core::Emulator;

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
        emulator: Emulator::new(&rom, None, nestadia_core::ExecutionMode::Ring3)?,
        is_running: true,
    }));

    sdl_window::start_game(emulation_state);
    Ok(())
}


// Copied from server/
use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

#[cfg(not(any(feature = "gui", feature = "server")))]
compile_error!("You need to select at least one feature!");

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(default_value = "info", short, long)]
    log_level: String,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[cfg(feature = "gui")]
    Gui {
        #[structopt(parse(from_os_str), default_value = "./default_roms/flappybird.nes")]
        rom: PathBuf,
    },
    #[cfg(feature = "server")]
    Server {
        #[structopt(default_value = "127.0.0.1", long, short)]
        bind_addr: String,
        #[structopt(default_value = "8080", long, short)]
        port: u16,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    flexi_logger::Logger::with_str(opt.log_level)
        .start()
        .unwrap();

    match opt.cmd {
        #[cfg(feature = "gui")]
        Command::Gui { rom } => gui::gui_start(rom)?,
        #[cfg(feature = "server")]
        Command::Server { bind_addr, port } => server::actix_main(bind_addr, port)?,
    }

    Ok(())
}

