#[macro_use]
extern crate libretro_backend;

extern crate nestadia;
extern crate bitflags;

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

pub struct State {
    emulator: Option<Emulator>,
    game_data: Option<GameData>,
}

impl State {
    fn new() -> State {
        State {
            emulator: None,
            game_data: None,
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

        if game_data.path().is_none() {
            return LoadGameResult::Failed(game_data);
        }

        // Get the path supplied and prepare it to be used as rom and save paths by the Core
        let mut path = PathBuf::new();
        path.set_file_name(game_data.path().unwrap());

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
        let data: &[u8] = match game_data.data() {
            None => {
                return LoadGameResult::Failed(game_data);
            },
            Some(data) => { data }
        };

        // Create emulator instance
        let emulator = Emulator::new(data, save_file).expect("Rom parsing failed");
        self.emulator = Some(emulator);
        self.game_data = Some(game_data);
        // return failed a la place

        // Create **dummy** AV info
        let av_info = AudioVideoInfo::new()
            .video( 256, 240, 60.00, PixelFormat::ARGB8888)
            .audio( 44100.0 )
            .region( Region::NTSC );

        return LoadGameResult::Success(av_info);
    }

    fn on_unload_game(&mut self) -> GameData {
        self.game_data.take().expect("Tried to unload a game while already being unloaded.") // erreur a été called 2 fois?
    }

    fn on_run(&mut self, handle: &mut RuntimeHandle) {
        
        match &mut self.emulator {
            Some(emulator) => {

                let frame = loop {
                    if let Some(frame) = emulator.clock() {
                        break frame;
                    }
                };
        
                let mut current_frame = [0u8; NUM_PIXELS * 4];
                nestadia::frame_to_argb(&frame, &mut current_frame);

                handle.upload_video_frame(&current_frame);
                handle.upload_audio_frame(&MOCK_AUDIO);
            },
            None => { }
        }

        // idk wtf im doing
        /*macro_rules! controller_input {
            ( $($button:ident), +) => {
                $(
                    let buttonpress_id = match JoypadButton::$button {
                        libretro_sys::DEVICE_ID_JOYPAD_A => ControllerState::A,
                        libretro_sys::DEVICE_ID_JOYPAD_B => ControllerState::B,
                        libretro_sys::DEVICE_ID_JOYPAD_START => ControllerState::START,
                        libretro_sys::DEVICE_ID_JOYPAD_SELECT => ControllerState::SELECT,
                        libretro_sys::DEVICE_ID_JOYPAD_LEFT => ControllerState::LEFT,
                        libretro_sys::DEVICE_ID_JOYPAD_RIGHT => ControllerState::RIGHT,
                        libretro_sys::DEVICE_ID_JOYPAD_UP => ControllerState::UP,
                        libretro_sys::DEVICE_ID_JOYPAD_DOWN => ControllerState::DOWN
                    };

                    self.emulator.unwrap().controller_input
                )+
            }
        }*/

    }

    fn on_reset(&mut self) {
        match &mut self.emulator {
            Some(emu) => {
                emu.reset();
            },
            None => { }
        }
    }
}

libretro_core!( State );