#[macro_use]
extern crate libretro_backend;

extern crate bitflags;
extern crate nestadia;

use bitflags::bitflags;
use std::vec::Vec;
use std::io::Read;
use std::path::PathBuf;
use nestadia::Emulator;
use libretro_backend::{CoreInfo, AudioVideoInfo, PixelFormat, GameData, LoadGameResult, Region, RuntimeHandle, JoypadButton};

// NES outputs a 256 x 240 pixel image
const NUM_PIXELS: usize = 256 * 240;

const MOCK_AUDIO: [i16; 1470] = [0i16; 1470];

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

impl ControllerState {
    fn from(button: JoypadButton) -> ControllerState {
        match button {
            JoypadButton::A => ControllerState::A,
            JoypadButton::B => ControllerState::B,
            JoypadButton::Start => ControllerState::START,
            JoypadButton::Select => ControllerState::SELECT,
            JoypadButton::Down => ControllerState::DOWN,
            JoypadButton::Left => ControllerState::LEFT,
            JoypadButton::Right => ControllerState::RIGHT,
            JoypadButton::Up => ControllerState::UP,
            _ => ControllerState::NONE
        }
    }
}

pub struct State {
    emulator: Option<Emulator>,
    game_data: Option<GameData>,
    controller1: ControllerState,
    controller2: ControllerState
}

impl State {
    fn new() -> State {
        State {
            emulator: None,
            game_data: None,
            controller1: ControllerState::NONE,
            controller2: ControllerState::NONE
        }
    }
}

impl Default for State {
    fn default() -> Self {
            
        return State::new();
    }
}

impl libretro_backend::Core for State {
    fn info() -> CoreInfo {
        CoreInfo::new("Nestadia", env!("CARGO_PKG_VERSION")).supports_roms_with_extension("nes")
    }

    fn on_load_game(&mut self, game_data: GameData) -> LoadGameResult {
        if game_data.is_empty() {
            return LoadGameResult::Failed(game_data);
        }

        let game_data_path = match game_data.path() {
            None => {
                return LoadGameResult::Failed(game_data);
            },
            Some(path) => { path }
        };

        // Get the path supplied and prepare it to be used as rom and save paths by the Core
        let mut path = PathBuf::new();
        path.set_file_name(game_data_path);

        let mut save_path = path.clone();
        save_path.set_extension("sav");

        // Read the save file
        let mut save_buf = Vec::new();
        let save_file = if let Ok(mut file) = std::fs::File::open(&save_path) {
            let _ = file.read_to_end(&mut save_buf);
            Some(save_buf.as_slice())
        } else {
            None
        };

        // Get the game data
        let data = match game_data.data() {
            None => {
                println!("Failed to retrieve game data");
                return LoadGameResult::Failed(game_data);
            },
            Some(data) => { data }
        };

        // Create emulator instance
        let emulator = match Emulator::new(data, save_file) {
            Err(_) => {
                println!("Rom parsing failed");
                return LoadGameResult::Failed(game_data);
            },
            Ok(emulator) => { emulator }
        };

        self.emulator = Some(emulator);
        self.game_data = Some(game_data);

        // This info is just what's expected and is static for now. We might need to change it later if need be.
        let av_info = AudioVideoInfo::new()
            .video( 256, 240, 60.00, PixelFormat::ARGB8888)
            .audio( 44100.0 )
            .region( Region::NTSC );

        return LoadGameResult::Success(av_info);
    }

    fn on_unload_game(&mut self) -> GameData {
        self.game_data.take().expect("Tried to unload a game while already being unloaded.") // l'erreur est callée 2 fois?
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {

        let emulator = match &mut self.emulator {
            Some(emulator) => {
                emulator
            },
            None => { return; }
        };

        let frame = loop {
            if let Some(frame) = emulator.clock() {
                break frame;
            }
        };

        let mut current_frame = [0u8; NUM_PIXELS * 4];
        nestadia::frame_to_argb(&frame, &mut current_frame);

        handle.upload_video_frame(&current_frame);
        handle.upload_audio_frame(&MOCK_AUDIO);

        // Reading controller inputs
        macro_rules! update_controllers {
            ( $( $button:ident ),+ ) => (
                $(
                    let controller_state: ControllerState = match ControllerState::from(JoypadButton::$button) {
                        ControllerState::NONE => { return; },
                        state => { state }
                    };
            
                    // Setting controller 1 button state
                    if handle.is_joypad_button_pressed(0, JoypadButton::$button) {
                        self.controller1 |= controller_state;
                    } else if self.controller1.contains(controller_state) {
            
                        self.controller1 &= !controller_state;
                    }

                    // Setting controller 2 button state
                    /*if handle.is_joypad_button_pressed(1, JoypadButton::$button) {
                        self.controller2 |= controller_state;
                    } else if self.controller2.contains(controller_state) {
            
                        self.controller2 &= !controller_state;
                    }*/
                )+
            )
        }

        update_controllers!(A, B, Up, Down, Left, Right, Select, Start);

        emulator.set_controller1(self.controller1.bits());
        // emulator.set_controller2(self.controller2.bits());
    }

    fn on_reset(&mut self) {
        match &mut self.emulator {
            Some(emu) => {
                emu.reset();
            },
            None => { }
        }
    }

    fn save_memory(&mut self) -> Option<&mut [u8]> {
        None // Not implemented
    }

    fn rtc_memory( &mut self ) -> Option< &mut [u8] > {
        None // Not implemented
    }

    fn system_memory( &mut self ) -> Option< &mut [u8] > {
        None // Not implemented
    }

    fn video_memory( &mut self ) -> Option< &mut [u8] > {
        None // Not implemented
    }
}

libretro_core!( State );