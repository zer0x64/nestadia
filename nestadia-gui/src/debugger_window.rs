use std::{
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use iced::{
    executor,
    keyboard::{self, KeyCode},
    scrollable, Application, Clipboard, Column, Command, Element, Row, Scrollable, Subscription,
    Text,
};

use nestadia::Emulator;

use super::{EmulationState, NES_HEIGHT, NES_WIDTH};

pub(crate) struct NestadiaIced {
    emulation_state: Arc<RwLock<EmulationState>>,
    scrollable_state: scrollable::State,
    disassembly: Vec<(u16, String)>,
}

#[derive(Default)]
pub(crate) struct NestadiaIcedRunFlags {
    pub rom_path: PathBuf,
}

#[derive(Debug)]
pub(crate) enum Message {
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
            emulator: Emulator::new(&rom, None).unwrap(),
            is_running: false,
        }));

        let emulation_state_sdl = emulation_state.clone();

        std::thread::spawn(move || {
            super::sdl_window::start_game(emulation_state_sdl);
        });

        (
            NestadiaIced {
                emulation_state,
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
                    emulation_state.emulator.cpu().cycles > 0
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
            image_data.extend_from_slice(match i % 30 {
                0..=9 => &[0xff, 0, 0, 0xff],
                10..=19 => &[0, 0xff, 0, 0xff],
                20..=29 => &[0, 0, 0xff, 0xff],
                _ => &[0xff, 0xff, 0xff, 0xff],
            });
        }

        let cpu = self.emulation_state.read().unwrap().emulator.cpu().clone();

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
                iced_native::Event::Keyboard(keyboard::Event::KeyPressed { key_code, .. }) => {
                    match key_code {
                        KeyCode::F7 => Some(Self::Message::Step),
                        KeyCode::F8 => Some(Self::Message::Disassemble),
                        KeyCode::Space => Some(Self::Message::PauseUnpause),
                        _ => None,
                    }
                }
                _ => None,
            });

        Subscription::batch(vec![keyboard_events])
    }
}
