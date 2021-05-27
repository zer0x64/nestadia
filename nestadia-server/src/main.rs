mod nestadia_ws;

use std::error::Error;

use structopt::StructOpt;

use nestadia_ws::{EmulationState, NestadiaWs};

use std::time::Instant;

use serde::{Deserialize, Serialize};

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

const ROM_LIST: [&str; 3] = ["Flappybird", "Alter Ego", "Nesert Bus"];

#[derive(Debug, Serialize, Deserialize)]
struct Credentials {
    password: String,
}

async fn emulator_start_param(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let rom_name = req.match_info().get("rom_name").unwrap();

    let rom: &[u8] = match rom_name {
        _ if rom_name == ROM_LIST[0] => include_bytes!("../../default_roms/flappybird.nes"),
        _ if rom_name == ROM_LIST[1] => include_bytes!("../../default_roms/Alter_Ego.nes"),
        _ if rom_name == ROM_LIST[2] => include_bytes!("../../default_roms/nesertbus.nes"),
        _ => return Ok(HttpResponse::NotFound().into()),
    };

    let websocket = NestadiaWs {
        state: EmulationState::Ready { rom: rom.to_vec() },
        heartbeat: Instant::now(),
        custom_rom: vec![],
        custom_rom_len: 0,
    };

    ws::start(websocket, &req, stream)
}

async fn custom_emulator(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs {
        state: EmulationState::Waiting,
        heartbeat: Instant::now(),
        custom_rom: vec![],
        custom_rom_len: 0,
    };

    ws::start(websocket, &req, stream)
}

async fn rom_list(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().json(ROM_LIST)
}

#[actix_web::main]
pub async fn actix_main(bind_addr: String, port: u16) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(
                web::scope("/api")
                    .route("/emulator/custom", web::get().to(custom_emulator))
                    .route("/emulator/{rom_name}", web::get().to(emulator_start_param))
                    .route("/list", web::get().to(rom_list)),
            )
            .service(
                actix_files::Files::new("/", "client_build")
                    .index_file("index.html")
                    .disable_content_disposition(),
            )
    })
    .bind((bind_addr, port))?
    .run()
    .await
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(default_value = "info", short, long)]
    log_level: String,

    #[structopt(default_value = "127.0.0.1", long, short)]
    bind_addr: String,

    #[structopt(default_value = "8080", long, short)]
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    flexi_logger::Logger::with_str(opt.log_level)
        .start()
        .unwrap();

    Ok(actix_main(opt.bind_addr, opt.port)?)
}
