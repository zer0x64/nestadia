use std::convert::TryInto;
use std::io::Write;
use std::{
    fs::{self, OpenOptions},
    io::Read,
    pin::Pin,
    sync::mpsc::{channel, Receiver, Sender},
    time::{Duration, Instant},
};

use futures::task::{Poll, Waker};
use log::info;

use actix::prelude::*;
use actix_web_actors::ws;
use flate2::{write::GzEncoder, Compression};

use nestadia_core::{Emulator, RomParserError};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy)]
pub struct EmulationError(RomParserError);

impl core::fmt::Display for EmulationError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl std::error::Error for EmulationError {}

pub enum EmulationState {
    Waiting,                        // wait for a user-provided ROM
    Ready { rom: Vec<u8> },         // ready to start immediately
    Started(Sender<EmulatorInput>), // up and running
}

pub struct NestadiaWs {
    pub state: EmulationState,
    pub heartbeat: Instant,
    pub custom_rom: Vec<u8>,
    pub custom_rom_len: usize,
}

struct FrameStream {
    receiver: Receiver<Vec<u8>>,
    sender: Sender<Waker>,
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
        ctx: &mut futures::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Check whether a frame is ready
        match self.receiver.try_recv() {
            Ok(f) => Poll::Ready(Some(Frame(f))),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                let _ = self.sender.send(ctx.waker().clone());
                Poll::Pending
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Poll::Ready(None),
        }
    }
}

impl Actor for NestadiaWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        if let EmulationState::Ready { rom } = &self.state {
            // At this point, ROMs are hardcoded, so this shouldn't fail
            let sender = start_emulation(ctx, rom).unwrap();
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
        // For clients that doesn't support Ping (looking at you Python)
        self.heartbeat = Instant::now();

        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),

            Ok(ws::Message::Pong(_)) => self.heartbeat = Instant::now(),

            // If we receive something here, it's the controller input.
            Ok(ws::Message::Binary(bin)) => {
                match &mut self.state {
                    EmulationState::Waiting => {
                        // Received chunk of ROM
                        if self.custom_rom.len() == 0 {
                            // First 4 bytes are used to specify the ROM's size
                            self.custom_rom_len =
                                u32::from_le_bytes(bin[0..4].try_into().unwrap()) as usize;
                            self.custom_rom = Vec::with_capacity(self.custom_rom_len);

                            self.custom_rom.extend_from_slice(&bin[4..]);
                        } else {
                            self.custom_rom.extend_from_slice(&bin);
                        }

                        if self.custom_rom.len() == self.custom_rom_len {
                            match start_emulation(ctx, &self.custom_rom) {
                                Ok(sender) => self.state = EmulationState::Started(sender),
                                // If there's an error, just ignore it and wait for a valid ROM
                                Err(_) => (),
                            }
                        }
                    }
                    EmulationState::Started(input_sender) => {
                        // Received controller input
                        if bin.len() > 0 {
                            let _ = input_sender.send(EmulatorInput::Controller1(bin[0]));
                        };
                    }
                    EmulationState::Ready { .. } => (), // Ignore
                }
            }
            Ok(ws::Message::Close(_)) => ctx.stop(),
            _ => (log::warn!("Websocket received msg of unsupported type {:?}", msg)),
        }
    }
}

impl Handler<Frame> for NestadiaWs {
    type Result = ();

    fn handle(&mut self, msg: Frame, ctx: &mut Self::Context) {
        // let mut rng = rand::thread_rng();
        // let mut new_frame: Vec<u8> = Vec::new();
        // for _ in 0..240 {
        //     let new_val: u8 = rng.gen_range(0..64);
        //     new_frame.extend(&[new_val; 256]);
        // }

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        match encoder.write_all(&msg.0) {
            Ok(_) => ctx.binary(encoder.finish().unwrap()), // TODO: Send real frame
            Err(_) => {} //Simply skip the frame if there's an error during compression
        }
    }
}

fn start_emulation(
    ctx: &mut ws::WebsocketContext<NestadiaWs>,
    rom: &[u8],
) -> Result<Sender<EmulatorInput>, Box<dyn std::error::Error>> {
    // Read save file
    let rom_hash = blake3::hash(rom).to_hex().to_string();
    let save_path = "saves/".to_string() + &rom_hash + ".save";
    let mut buf = Vec::new();

    let save_data = if let Ok(mut f) = std::fs::File::open(&save_path) {
        let _ = f.read_to_end(&mut buf);
        Some(buf.as_slice())
    } else {
        None
    };

    let mut emulator = Emulator::new(rom, save_data).map_err(EmulationError)?;

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

        // Save file
        if let Err(e) = fs::create_dir_all("saves") {
            log::warn!("Couldn't create save folder: {}", e)
        };

        if let Some(save_data) = emulator.get_save_data() {
            if let Ok(mut f) = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(save_path)
            {
                let _ = f.write_all(save_data);
            }
        }
    });

    ctx.add_message_stream(FrameStream {
        receiver: frame_receiver,
        sender: waker_sender,
    });

    Ok(input_sender)
}
