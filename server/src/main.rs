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
        #[structopt(parse(from_os_str), default_value = "./test_roms/Donkey Kong.nes")]
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
