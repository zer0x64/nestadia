use std::{
    pin::Pin,
    sync::mpsc::{channel, Receiver, Sender},
};

use futures::task::Poll;

use actix::prelude::*;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;

use nestadia_core::Emulator;

struct NestadiaWs {
    input_sender: Sender<(bool, u8)>,

    // Stored here temporarly until moved in the stream in started()
    frame_receiver: Option<Box<Receiver<Vec<u8>>>>,
}

struct FrameStream {
    receiver: Box<Receiver<Vec<u8>>>,
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
    pub fn new(rom: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut emulator = Emulator::new(rom)?;

        let (input_sender, input_receiver) = channel();
        let (frame_sender, frame_receiver) = channel();

        // This thread runs the actual emulator and sync the framerate
        std::thread::spawn(move || {
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
                };

                match frame_sender.send(frame.to_vec()) {
                    Ok(_) => {}
                    Err(_) => break, // Stop the thread if there is an error to avoid infinite loop
                };
            }
        });

        Ok(NestadiaWs {
            input_sender,
            frame_receiver: Some(Box::new(frame_receiver)),
        })
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Here we should create the stream and register it to the actor.
        let frame_receiver = self.frame_receiver.take().unwrap();
        self.frame_receiver = None;

        let stream = FrameStream {
            receiver: frame_receiver,
        };
        ctx.add_message_stream(stream);
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

impl Handler<Frame> for NestadiaWs {
    type Result = ();

    fn handle(&mut self, msg: Frame, ctx: &mut Self::Context) {
        ctx.binary(msg.0)
    }
}

async fn emulator_start(req: HttpRequest, stream: web::Payload) -> impl Responder {
    let websocket = NestadiaWs::new(include_bytes!("../../test_roms/1.Branch_Basics.nes"));

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
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
