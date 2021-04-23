use actix::{Actor, StreamHandler};
use actix_web::{
    web::{self, Bytes},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

use nestadia_core::Emulator;

struct NestadiaWs {
    emulator: Emulator,
}

impl NestadiaWs {
    pub fn new(rom: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(NestadiaWs {
            emulator: Emulator::new(rom)?,
        })
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NestadiaWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn emulator(req: HttpRequest, stream: web::Payload, body: Bytes) -> impl Responder {
    let websocket = NestadiaWs::new(body.as_ref());

    match websocket {
        Ok(websocket) => ws::start(websocket, &req, stream),
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

#[actix_web::main]
pub async fn actix_main(port: u16) -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/emulator", web::post().to(emulator)))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
