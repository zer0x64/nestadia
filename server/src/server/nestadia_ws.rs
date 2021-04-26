use std::{
    pin::Pin,
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use futures::task::{Poll, Waker};
use log::info;

use actix::prelude::*;
use actix_web_actors::ws;
use flate2::{write::GzEncoder, Compression};

use nestadia_core::{Emulator, ExecutionMode};
use rand::Rng;
use std::io::Write;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

pub enum EmulationState {
    Waiting {
        exec_mode: ExecutionMode,
    }, // wait for a user-provided ROM
    Ready {
        rom: &'static [u8],
        exec_mode: ExecutionMode,
    }, // ready to start immediately
    Started(Sender<EmulatorInput>), // up and running
}

pub struct NestadiaWs {
    pub state: EmulationState,
    pub heartbeat: Instant,
}

struct FrameStream {
    receiver: Receiver<Vec<u8>>,
    sender: Sender<Waker>,
    last_poll: Instant,
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
        mut self: Pin<&mut Self>,
        ctx: &mut futures::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // println!("poll {:?}", Instant::now() - self.last_poll);

        self.last_poll = Instant::now();

        // Check whether a frame is ready
        match self.receiver.try_recv() {
            Ok(f) => Poll::Ready(Some(Frame(f))),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                self.sender.send(ctx.waker().clone());
                Poll::Pending
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let EmulationState::Ready { rom, exec_mode } = &self.state {
            // At this point, ROMs are hardcoded, so this shouldn't fail
            let sender = start_emulation(ctx, rom, *exec_mode).unwrap();
            self.state = EmulationState::Started(sender);
        }

        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.heartbeat) > CLIENT_TIMEOUT {
                info!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
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
                    EmulationState::Waiting { exec_mode } => {
                        match start_emulation(ctx, &bin, *exec_mode) {
                            Ok(sender) => self.state = EmulationState::Started(sender),
                            // If there's an error, just ignore it and wait for a valid ROM
                            Err(_) => (),
                        }
                    } // Received ROM
                    EmulationState::Started(input_sender) => input_sender
                        .send(EmulatorInput::Controller1(bin[0]))
                        .unwrap(), // TODO: plz handle result
                    EmulationState::Ready { .. } => (), // Ignore
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
        let mut rng = rand::thread_rng();
        let mut new_frame: Vec<u8> = Vec::new();
        for _ in 0..240 {
            let new_val: u8 = rng.gen_range(0..64);
            new_frame.extend(&[new_val; 256]);
        }

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&new_frame);

        ctx.binary(encoder.finish().unwrap()) // TODO: Send real frame
    }
}

fn start_emulation(
    ctx: &mut ws::WebsocketContext<NestadiaWs>,
    rom: &[u8],
    execution_mode: ExecutionMode,
) -> Result<Sender<EmulatorInput>, Box<dyn std::error::Error>> {
    let mut emulator = Emulator::new(rom, execution_mode)?;

    let (input_sender, input_receiver) = channel();
    let (frame_sender, frame_receiver) = channel();
    let (waker_sender, waker_receiver) = channel();

    // This thread runs the actual emulator and sync the framerate
    std::thread::spawn(move || {
        let mut next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);
        let mut frame_waker: Option<Waker> = None;

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
                if let Some(frame) = emulator.clock() {
                    break frame;
                }
            }
            .to_vec();

            if Instant::now() < next_frame_time {
                std::thread::sleep(next_frame_time.duration_since(Instant::now()));
            };

            match frame_sender.send(frame) {
                Ok(_) => {}
                Err(_) => break, // Stop the thread if there is an error to avoid infinite loop
            };

            // Wake the FrameStream task
            if let Ok(waker) = waker_receiver.try_recv() {
                frame_waker = Some(waker);
            };
            if let Some(waker) = frame_waker.take() {
                waker.wake();
            }

            next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);
        }
    });

    ctx.add_message_stream(FrameStream {
        receiver: frame_receiver,
        sender: waker_sender,
        last_poll: Instant::now(),
    });

    Ok(input_sender)
}
