use std::sync::mpsc::{channel, Receiver, Sender};

use actix::prelude::*;
use actix_web::{
    web::{self, Bytes},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

use nestadia_core::Emulator;

struct NestadiaWs {
    input_sender: Sender<(bool, u8)>,
    frame_receiver: Receiver<[u8; 256 * 240]>,
}

impl NestadiaWs {
    pub fn new(rom: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut emulator = Emulator::new(rom)?;

        // Kind of a placeholder for now, will need to implement a Stream with this instead and plug it into the actor
        let (input_sender, input_receiver) = channel();
        let (frame_sender, frame_receiver) = channel();

        // This thread runs the actual emulator and sync the framerate
        std::thread::spawn(move || {
            loop {
                if let Ok((stop, input)) = input_receiver.recv() {
                    if stop {
                        break;
                    }

                    emulator.set_controller1(input);
                }

                let frame = loop {
                    match emulator.clock() {
                        Some(frame) => break frame,
                        None => {}
                    }
                };

                frame_sender.send(frame).unwrap(); // TODO: plz handle error
            }
        });

        Ok(NestadiaWs {
            input_sender,
            frame_receiver,
        })
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Here we should create the stream and register it to the actor.
        // ctx.add_stream()
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Tell the emulation thread to stop
        self.input_sender.send((true, 0)).unwrap()
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NestadiaWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),

            // If we receive something here, it's the controller input.
            Ok(ws::Message::Binary(bin)) => self.input_sender.send((false, bin[0])).unwrap(), // TODO: plz handle result
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => (),
        }
    }
}

async fn emulator_start(req: HttpRequest, stream: web::Payload, body: Bytes) -> impl Responder {
    let websocket = NestadiaWs::new(body.as_ref());

    match websocket {
        Ok(websocket) => ws::start(websocket, &req, stream),
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

#[actix_web::main]
pub async fn actix_main(port: u16) -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/emulator", web::post().to(emulator_start)))
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
