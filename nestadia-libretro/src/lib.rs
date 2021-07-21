#[macro_use]
extern crate libretro_backend;

extern crate bitflags;
extern crate nestadia;

use bitflags::bitflags;
use libretro_backend::{
    AudioVideoInfo, Core, CoreInfo, GameData, JoypadButton, LoadGameResult, PixelFormat, Region,
    RuntimeHandle,
};
use nestadia::Emulator;

// NES outputs a 256 x 240 pixel image
const NUM_PIXELS: usize = 256 * 240;

bitflags! {
    #[derive(Default)]
    struct ControllerState: u8 {
        const NONE = 0x00;
        const A = 0x80;
        const B = 0x40;
        const SELECT = 0x20;
        const START = 0x10;
        const UP = 0x08;
        const DOWN = 0x04;
        const LEFT = 0x02;
        const RIGHT = 0x01;
    }
}

impl From<JoypadButton> for ControllerState {
    fn from(button: JoypadButton) -> Self {
        match button {
            JoypadButton::A => Self::A,
            JoypadButton::B => Self::B,
            JoypadButton::Start => Self::START,
            JoypadButton::Select => Self::SELECT,
            JoypadButton::Down => Self::DOWN,
            JoypadButton::Left => Self::LEFT,
            JoypadButton::Right => Self::RIGHT,
            JoypadButton::Up => Self::UP,
            _ => Self::NONE,
        }
    }
}

pub struct State {
    emulator: Option<Emulator>,
    game_data: Option<GameData>,
    controller1: ControllerState,
    controller2: ControllerState,
}

impl State {
    fn new() -> State {
        State {
            emulator: None,
            game_data: None,
            controller1: ControllerState::NONE,
            controller2: ControllerState::NONE,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}

impl Core for State {
    fn info() -> CoreInfo {
        CoreInfo::new("Nestadia", env!("CARGO_PKG_VERSION")).supports_roms_with_extension("nes")
    }

    fn on_load_game(&mut self, game_data: GameData) -> LoadGameResult {
        if game_data.is_empty() {
            return LoadGameResult::Failed(game_data);
        }

        // Note: libretro seems to give both the option to get the game's data from data() and
        // from a path to the file from which we could read the data ourselves. For now, I just get it from data().

        // Get the rom data
        let rom_data = match game_data.data() {
            None => {
                return LoadGameResult::Failed(game_data);
            }
            Some(data) => data,
        };

        // Get the save data TODO
        let save_data = None;

        // Create emulator instance
        let emulator = match Emulator::new(rom_data, save_data) {
            Err(_) => {
                return LoadGameResult::Failed(game_data);
            }
            Ok(emulator) => emulator,
        };

        self.emulator = Some(emulator);
        self.game_data = Some(game_data);

        // This info is just what's expected and is all hard coded for now.
        // We might need to change it later if need be.
        let av_info = AudioVideoInfo::new()
            .video(256, 240, 60.00, PixelFormat::ARGB8888)
            .audio(44100.0)
            .region(Region::NTSC);

        LoadGameResult::Success(av_info)
    }

    fn on_unload_game(&mut self) -> GameData {
        self.game_data
            .take()
            .expect("Tried to unload a game while already being unloaded.")
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {
        let emulator = match &mut self.emulator {
            None => {
                return;
            }
            Some(emulator) => emulator,
        };

        let frame = loop {
            if let Some(frame) = emulator.clock() {
                break frame;
            }
        };

        let mut current_frame = [0u8; NUM_PIXELS * 4];
        nestadia::frame_to_argb(&frame, &mut current_frame);

        handle.upload_video_frame(&current_frame);

        let mut audio_buffer = Vec::with_capacity(2048);
        audio_buffer.extend(
            emulator.take_audio_samples().flat_map(|sample| {
                let sample = if sample >= 1.0 {
                    32767
                } else if sample <= -1.0 {
                    -32768
                } else {
                    (sample * 32767.0) as i16
                };

                // Duplicate the value to transform mono audio to stereo
                [sample, sample]
            })
        );

        // NOTE: When rounding CPU_CYCLES_PER_SAMPLE (using +0.5) to 41, there is 1454 frames
        // so 16 frames missing. Without rounding (so 40 cpu cycles to create a sample), it generates
        // 1490 frames, so it has enough samples. The actual accurate value is 40.5, but cpu cycles
        // are not floats.
        println!("Audio buffer length: {}", audio_buffer.len());

        if audio_buffer.len() < 1470 {
            for _ in 0..(1470 - audio_buffer.len()) {
                audio_buffer.push(*audio_buffer.last().unwrap());
            }
        }

        handle.upload_audio_frame(&audio_buffer[..]);

        // Reading controller inputs
        macro_rules! update_controllers {
            ( $( $button:ident ),+ ) => (
                $(
                    let controller_state: ControllerState = match ControllerState::from(JoypadButton::$button) {
                        ControllerState::NONE => { return; },
                        state => state
                    };

                    // Setting controller 1 button state
                    if handle.is_joypad_button_pressed(0, JoypadButton::$button) {
                        self.controller1 |= controller_state;

                    } else if self.controller1.contains(controller_state) {
                        self.controller1 &= !controller_state;
                    }

                    // Setting controller 2 button state
                    if handle.is_joypad_button_pressed(1, JoypadButton::$button) {
                        self.controller2 |= controller_state;

                    } else if self.controller2.contains(controller_state) {
                        self.controller2 &= !controller_state;
                    }
                )+
            )
        }

        update_controllers!(A, B, Up, Down, Left, Right, Select, Start);

        emulator.set_controller1(self.controller1.bits());
        emulator.set_controller2(self.controller2.bits());
    }

    fn on_reset(&mut self) {
        match &mut self.emulator {
            None => {}
            Some(emu) => {
                emu.reset();
            }
        }
    }
}

libretro_core!(State);
