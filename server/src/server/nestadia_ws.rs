use std::{
    pin::Pin,
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use futures::task::Poll;
use log::info;

use actix::prelude::*;
use actix_web_actors::ws;

use nestadia_core::{Emulator, ExecutionMode};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

pub enum EmulationState {
    NotStarted((Option<&'static [u8]>, ExecutionMode)),
    Started(Sender<EmulatorInput>),
}

pub struct NestadiaWs {
    pub state: EmulationState,
    pub heartbeat: Instant,
}

struct FrameStream {
    receiver: Receiver<Vec<u8>>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Frame(Vec<u8>);

pub enum EmulatorInput {
    Stop,
    Controller1(u8),
}

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
    fn start_emulation(
        &mut self,
        ctx: &mut ws::WebsocketContext<Self>,
        rom: &[u8],
        execution_mode: ExecutionMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut emulator = Emulator::new(rom, execution_mode)?;

        let (input_sender, input_receiver) = channel();
        let (frame_sender, frame_receiver) = channel();

        // This thread runs the actual emulator and sync the framerate
        std::thread::spawn(move || {
            let mut next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);

            loop {
                // Check if we received  an input or if we close the thread
                if let Ok(emulator_input) = input_receiver.try_recv() {
                    match emulator_input {
                        EmulatorInput::Stop => break,
                        EmulatorInput::Controller1(x) => emulator.set_controller1(x),
                    }
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
        ctx.add_message_stream(FrameStream {
            receiver: frame_receiver,
        });

        Ok(())
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let EmulationState::NotStarted((Some(rom), e)) = &self.state {
            let e = e.clone();

            // ROMs are hardcoded, so this shouldn't fail
            self.start_emulation(ctx, &rom, e).unwrap()
        }

        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                info!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            } else {
                ctx.ping(b"");
            }
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // Tell the emulation thread to stop
        if let EmulationState::Started(input_sender) = &self.state {
            input_sender.send(EmulatorInput::Stop).unwrap()
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NestadiaWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),

            Ok(ws::Message::Pong(_)) => self.heartbeat = Instant::now(),

            // If we receive something here, it's the controller input.
            Ok(ws::Message::Binary(bin)) => {
                match &mut self.state {
                    EmulationState::NotStarted((None, e)) => {
                        let e = e.clone();

                        // If there's an error, just ignore it and wait for a valid ROM
                        let _ = self.start_emulation(ctx, &bin, e);
                    } // Received ROM
                    EmulationState::Started(input_sender) => input_sender
                        .send(EmulatorInput::Controller1(bin[0]))
                        .unwrap(), // TODO: plz handle result
                    EmulationState::NotStarted((Some(_), _)) => (), // Ignore
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
