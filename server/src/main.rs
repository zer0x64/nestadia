use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

#[cfg(not(any(feature = "gui", feature = "server")))]
compile_error!("You need to select at least one feature!");

#[cfg(feature = "gui")]
mod gui;

#[cfg(feature = "server")]
mod server;

#[derive(Debug, StructOpt)]
enum Opt {
    #[cfg(feature = "gui")]
    Gui {
        #[structopt(parse(from_os_str), default_value = "./test_roms/Donkey Kong.nes")]
        rom: PathBuf,
    },
    #[cfg(feature = "server")]
    Server {
        #[structopt(default_value = "8080", long, short)]
        port: u16,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    flexi_logger::Logger::with_str("info").start().unwrap();

    match opt {
        #[cfg(feature = "gui")]
        Opt::Gui { rom } => gui::gui_start(rom)?,
        #[cfg(feature = "server")]
        Opt::Server { port } => server::actix_main(port)?,
    }

    Ok(())
}
