use std::{
    pin::Pin,
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use futures::task::Poll;

use actix::prelude::*;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use nestadia_core::Emulator;
use crate::server::EmulationState::NotStarted;

enum EmulationState {
    NotStarted(Option<&'static [u8]>),
    Started(Sender<(bool, u8)>),
}

struct NestadiaWs {
    state: EmulationState,
}

struct FrameStream {
    receiver: Receiver<Vec<u8>>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Frame(Vec<u8>);

impl Stream for FrameStream {
    type Item = Frame;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut futures::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Check whether a frame is ready
        match self.receiver.try_recv() {
            Ok(f) => Poll::Ready(Some(Frame(f))),
            Err(std::sync::mpsc::TryRecvError::Empty) => Poll::Pending,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}

impl NestadiaWs {
    // pub fn new(rom: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
    pub fn new(rom: Option<&'static [u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(NestadiaWs { state: NotStarted(rom) })
    }

    fn start_emulation(&mut self, ctx: &mut ws::WebsocketContext<Self>, rom: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut emulator = Emulator::new(rom)?;

        let (input_sender, input_receiver) = channel();
        let (frame_sender, frame_receiver) = channel();

        ctx.add_message_stream(FrameStream { receiver: frame_receiver });

        // This thread runs the actual emulator and sync the framerate
        std::thread::spawn(move || {
            let mut next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);

            loop {
                // Check if we received  an input or if we close the thread
                if let Ok((stop, input)) = input_receiver.try_recv() {
                    if stop {
                        break;
                    }

                    emulator.set_controller1(input);
                };

                // Loop until we get a frame
                let frame = loop {
                    match emulator.clock() {
                        Some(frame) => break frame,
                        None => {}
                    }
                }
                    .to_vec();

                if Instant::now() < next_frame_time {
                    ::std::thread::sleep(next_frame_time.duration_since(Instant::now()));
                };

                match frame_sender.send(frame) {
                    Ok(_) => {}
                    Err(_) => break, // Stop the thread if there is an error to avoid infinite loop
                };

                next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);
            }
        });

        self.state = EmulationState::Started(input_sender);

        Ok(())
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let EmulationState::NotStarted(Some(rom)) = self.state {
            self.start_emulation(ctx, &rom);
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Tell the emulation thread to stop
        if let EmulationState::Started(input_sender) = &self.state {
            input_sender.send((true, 0)).unwrap()
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NestadiaWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),

            // If we receive something here, it's the controller input.
            Ok(ws::Message::Binary(bin)) => {
                match &self.state {
                    EmulationState::NotStarted(None) => {
                        self.start_emulation(ctx, &bin);
                    } // Received ROM
                    EmulationState::Started(input_sender) => input_sender.send((false, bin[0])).unwrap(), // TODO: plz handle result
                    EmulationState::NotStarted(Some(_)) => (), // Ignore
                }
            }
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => (),
        }
    }
}

impl Handler<Frame> for NestadiaWs {
    type Result = ();

    fn handle(&mut self, msg: Frame, ctx: &mut Self::Context) {
        ctx.binary(msg.0)
    }
}

async fn emulator_start(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs::new(Some(include_bytes!("../../test_roms/1.Branch_Basics.nes")));

    match websocket {
        Ok(websocket) => ws::start(websocket, &req, stream),
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

async fn emulator_start_param(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let rom_name = req.match_info().get("rom_name").unwrap();

    let rom = match rom_name {
        "rom1" => include_bytes!("../../test_roms/1.Branch_Basics.nes"),
        "rom2" => include_bytes!("../../test_roms/2.Backward_Branch.nes"),
        "rom3" => include_bytes!("../../test_roms/3.Forward_Branch.nes"),
        _ => return Ok(HttpResponse::NotFound().into()),
    };

    let websocket = NestadiaWs::new(Some(rom));

    match websocket {
        Ok(websocket) => ws::start(websocket, &req, stream),
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

async fn custom_emulator(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs::new(None);

    match websocket {
        Ok(websocket) => ws::start(websocket, &req, stream),
        Err(e) => Ok(HttpResponse::BadRequest().body(e.to_string())),
    }
}

#[actix_web::main]
pub async fn actix_main(port: u16) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .route("/emulator", web::get().to(emulator_start))
            .service(web::scope("/api")
                .route("/emulator/custom", web::get().to(custom_emulator))
                .route("/emulator/{rom_name}", web::get().to(emulator_start_param))
            )
    })
        .bind(("127.0.0.1", port))?
        .run()
        .await
}
