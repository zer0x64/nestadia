use std::{fs, path::PathBuf, sync::{Arc, RwLock}, time::{Duration, Instant}};

use iced::{
    executor,
    keyboard::{self, KeyCode},
    scrollable, Application, Clipboard, Column, Command, Element, Row, Scrollable, Settings,
    Subscription, Text,
};
use iced_native;
use nestadia_core::Emulator;
use sdl2::{event::Event, keyboard::Keycode};

use structopt::StructOpt;

const NES_WIDTH: u32 = 256;
const NES_HEIGHT: u32 = 240;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str), default_value = "./test_roms/Donkey Kong.nes")]
    rom: PathBuf,
}

struct NestadiaIced {
    emulation_state: Arc<RwLock<EmulationState>>,
    scrollable_state: scrollable::State,
    disassembly: Vec<(u16, String)>,
}

#[derive(Default)]
struct NestadiaIcedRunFlags {
    rom_path: PathBuf,
}

struct EmulationState {
    emulator: Emulator,
    is_running: bool,
}

#[derive(Debug)]
enum Message {
    Step,
    PauseUnpause,
    Disassemble,
}

impl Application for NestadiaIced {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = NestadiaIcedRunFlags;

    fn new(flags: NestadiaIcedRunFlags) -> (NestadiaIced, Command<Self::Message>) {
        let rom = fs::read(flags.rom_path).unwrap();
        let emulation_state = Arc::new(RwLock::new(EmulationState {
            emulator: Emulator::new(&rom).unwrap(),
            is_running: false,
        }));

        let emulation_state_sdl = emulation_state.clone();

        std::thread::spawn(move || {
            let emulation_state = emulation_state_sdl;
            let sdl_context = sdl2::init().unwrap();
            let video_subsystem = sdl_context.video().unwrap();

            let window = video_subsystem
                .window("NEStadia", NES_WIDTH, NES_HEIGHT)
                .vulkan()
                .resizable()
                .position_centered()
                .build()
                .unwrap();

            let mut canvas = window.into_canvas().build().unwrap();

            let texture_creator = canvas.texture_creator();
            let mut texture = texture_creator
                .create_texture_streaming(
                    Some(sdl2::pixels::PixelFormatEnum::RGB24),
                    NES_WIDTH,
                    NES_HEIGHT,
                )
                .unwrap();

            let mut event_pump = sdl_context.event_pump().unwrap();

            emulation_state.write().unwrap().is_running = true;
            let mut i: u8 = 0;
            let mut next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);

            'running: loop {
                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. }
                        | Event::KeyDown {
                            keycode: Some(Keycode::Escape),
                            ..
                        } => break 'running,
                        _ => {}
                    }
                }

                if emulation_state.read().unwrap().is_running {
                    i = i.wrapping_add(1);
                    let mut emulation_state = emulation_state.write().unwrap();
                    while {
                        emulation_state.emulator.clock();
                        emulation_state.emulator.cpu.cycles > 0
                    } {}
                };

                texture.update(None, &[i; 256 * 240 * 3], 256).unwrap();

                canvas.copy(&texture, None, None).unwrap();
                canvas.present();

                if Instant::now() < next_frame_time {
                    ::std::thread::sleep(next_frame_time.duration_since(Instant::now()));
                };

                next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);
            }
        });
        (
            NestadiaIced {
                emulation_state: emulation_state,
                scrollable_state: scrollable::State::new(),
                disassembly: Vec::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Debugger")
    }

    fn update(&mut self, message: Self::Message, _: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::Step => {
                let mut emulation_state = self.emulation_state.write().unwrap();
                while {
                    emulation_state.emulator.clock();
                    emulation_state.emulator.cpu.cycles > 0
                } {}
            }
            Message::PauseUnpause => {
                let new_state = !self.emulation_state.read().unwrap().is_running;
                self.emulation_state.write().unwrap().is_running = new_state;
            }
            Message::Disassemble => {
                self.disassembly = self
                    .emulation_state
                    .read()
                    .unwrap()
                    .emulator
                    .disassemble(0, 0)
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<Self::Message> {
        let mut image_data = Vec::new();

        for i in 0..(NES_HEIGHT * NES_WIDTH) {
            image_data.extend_from_slice(&match i % 30 {
                0..=9 => [0xff, 0, 0, 0xff],
                1..=19 => [0, 0xff, 0, 0xff],
                2..=29 => [0, 0, 0xff, 0xff],
                _ => [0xff, 0xff, 0xff, 0xff],
            });
        }

        let cpu = self.emulation_state.read().unwrap().emulator.cpu.clone();

        // Filter the disassembly to show only part of it
        let mut disassembly = Vec::new();
        for (addr, disas) in &self.disassembly {
            if *addr as usize > cpu.pc as usize - 100 && (*addr as usize) < cpu.pc as usize + 100 {
                disassembly.push((addr, disas));
            }
        }

        // Sort the disassembly
        disassembly.sort_by_key(|d| d.0);

        // Get the widgets
        let mut disassembly_text = Vec::new();

        for (addr, disas) in disassembly {
            // Color it red if it is the next instruction
            let (color, size) = if *addr == cpu.pc {
                ([1.0, 0.0, 0.0], 20)
            } else {
                ([0.0, 0.0, 0.0], 12)
            };

            disassembly_text.push(
                Text::new(format!("{:#x}: {}", addr, disas))
                    .color(color)
                    .size(size)
                    .into(),
            );
        }

        let disassembly_window = Scrollable::new(&mut self.scrollable_state)
            .push(Column::with_children(disassembly_text));

        // The debugger window
        let debugger_window = Row::new().push(disassembly_window).push(
            Column::new()
                .push(Text::new(&format!("a: {:#x}", cpu.a)))
                .push(Text::new(&format!("x: {:#x}", cpu.x)))
                .push(Text::new(&format!("y: {:#x}", cpu.y)))
                .push(Text::new(&format!("st: {:#x}", cpu.st)))
                .push(Text::new(&format!("pc: {:#x}", cpu.pc)))
                .push(Text::new(&format!("status: {:#x}", cpu.status_register))),
        );

        debugger_window.into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let keyboard_events =
            iced_native::subscription::events_with(|event, _status| match event {
                iced_native::Event::Keyboard(event) => match event {
                    keyboard::Event::KeyPressed { key_code, .. } => match key_code {
                        KeyCode::F7 => Some(Self::Message::Step),
                        KeyCode::F8 => Some(Self::Message::Disassemble),
                        KeyCode::Space => Some(Self::Message::PauseUnpause),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            });

        Subscription::batch(vec![keyboard_events])
    }
}

fn main() -> iced::Result {
    let opt = Opt::from_args();
    flexi_logger::Logger::with_str("info").start().unwrap();

    NestadiaIced::run(Settings {
        window: iced::window::Settings {
            min_size: Some((NES_WIDTH as u32, NES_HEIGHT as u32)),
            ..Default::default()
        },
        flags: NestadiaIcedRunFlags { rom_path: opt.rom },
        ..Default::default()
    })
}
