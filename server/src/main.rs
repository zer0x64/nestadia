use std::error::Error;
use std::path::PathBuf;

use structopt::StructOpt;

mod gui;
mod server;

#[derive(Debug, StructOpt)]
enum Opt {
    Gui {
        #[structopt(parse(from_os_str), default_value = "./test_roms/Donkey Kong.nes")]
        rom: PathBuf,
    },
    Server {
        #[structopt(default_value = "8080", long, short)]
        port: u16,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    flexi_logger::Logger::with_str("info").start().unwrap();

    match opt {
        Opt::Gui { rom } => {
            gui::gui_start(rom)?;
        }
        Opt::Server { port } => {
            server::actix_main(port)?;
        }
    };

    Ok(())
}
