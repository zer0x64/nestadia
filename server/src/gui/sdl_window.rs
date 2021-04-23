use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use sdl2::{event::Event, keyboard::Keycode};

use super::{EmulationState, NES_HEIGHT, NES_WIDTH};

pub(crate) fn start_game(emulation_state: Arc<RwLock<EmulationState>>) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut controller_state = 0;

    let window = video_subsystem
        .window("NEStadia", NES_WIDTH, NES_HEIGHT)
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
    let mut next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::X),
                    ..
                } => controller_state |= 0x80,
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    ..
                } => controller_state |= 0x40,
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => controller_state |= 0x20,
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => controller_state |= 0x10,
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    ..
                } => controller_state |= 0x08,
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    ..
                } => controller_state |= 0x04,
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    ..
                } => controller_state |= 0x02,
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    ..
                } => controller_state |= 0x01,
                Event::KeyUp {
                    keycode: Some(Keycode::X),
                    ..
                } => controller_state |= 0x80,
                Event::KeyUp {
                    keycode: Some(Keycode::Z),
                    ..
                } => controller_state |= 0x40,
                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => controller_state &= !0x20,
                Event::KeyUp {
                    keycode: Some(Keycode::S),
                    ..
                } => controller_state &= !0x10,
                Event::KeyUp {
                    keycode: Some(Keycode::Up),
                    ..
                } => controller_state &= !0x08,
                Event::KeyUp {
                    keycode: Some(Keycode::Down),
                    ..
                } => controller_state &= !0x04,
                Event::KeyUp {
                    keycode: Some(Keycode::Left),
                    ..
                } => controller_state &= !0x02,
                Event::KeyUp {
                    keycode: Some(Keycode::Right),
                    ..
                } => controller_state &= !0x01,
                _ => {}
            }
        }

        if emulation_state.read().unwrap().is_running {
            let mut emulation_state = emulation_state.write().unwrap();
            emulation_state.emulator.set_controller1(controller_state);

            let frame = loop {
                match emulation_state.emulator.clock() {
                    Some(frame) => break frame,
                    None => {}
                }
            };

            // Maps 6 bit colors to RGB
            let frame: Vec<u8> = frame
                .iter()
                .flat_map(|c| {
                    match c {
                        0x00 => vec![0x7C, 0x7C, 0x7C],
                        0x01 => vec![0x00, 0x00, 0xFC],
                        0x02 => vec![0x00, 0x00, 0xBC],
                        0x03 => vec![0x44, 0x28, 0xBC],
                        0x04 => vec![0x94, 0x00, 0x84],
                        0x05 => vec![0xA8, 0x00, 0x20],
                        0x06 => vec![0xA8, 0x10, 0x00],
                        0x07 => vec![0x88, 0x14, 0x00],
                        0x08 => vec![0x50, 0x30, 0x00],
                        0x09 => vec![0x00, 0x78, 0x00],
                        0x0a => vec![0x00, 0x68, 0x00],
                        0x0b => vec![0x00, 0x58, 0x00],
                        0x0c => vec![0x00, 0x40, 0x58],
                        0x0d => vec![0x00, 0x00, 0x00],
                        0x0e => vec![0x00, 0x00, 0x00],
                        0x0f => vec![0x00, 0x00, 0x00],
                        0x10 => vec![0xBC, 0xBC, 0xBC],
                        0x11 => vec![0x00, 0x78, 0xF8],
                        0x12 => vec![0x00, 0x58, 0xF8],
                        0x13 => vec![0x68, 0x44, 0xFC],
                        0x14 => vec![0xD8, 0x00, 0xCC],
                        0x15 => vec![0xE4, 0x00, 0x58],
                        0x16 => vec![0xF8, 0x38, 0x00],
                        0x17 => vec![0xE4, 0x5C, 0x10],
                        0x18 => vec![0xAC, 0x7C, 0x00],
                        0x19 => vec![0x00, 0xB8, 0x00],
                        0x1a => vec![0x00, 0xA8, 0x00],
                        0x1b => vec![0x00, 0xA8, 0x44],
                        0x1c => vec![0x00, 0x88, 0x88],
                        0x1d => vec![0x00, 0x00, 0x00],
                        0x1e => vec![0x00, 0x00, 0x00],
                        0x1f => vec![0x00, 0x00, 0x00],
                        0x20 => vec![0xF8, 0xF8, 0xF8],
                        0x21 => vec![0x3C, 0xBC, 0xFC],
                        0x22 => vec![0x68, 0x88, 0xFC],
                        0x23 => vec![0x98, 0x78, 0xF8],
                        0x24 => vec![0xF8, 0x78, 0xF8],
                        0x25 => vec![0xF8, 0x58, 0x98],
                        0x26 => vec![0xF8, 0x78, 0x58],
                        0x27 => vec![0xFC, 0xA0, 0x44],
                        0x28 => vec![0xF8, 0xB8, 0x00],
                        0x29 => vec![0xB8, 0xF8, 0x18],
                        0x2a => vec![0x58, 0xD8, 0x54],
                        0x2b => vec![0x58, 0xF8, 0x98],
                        0x2c => vec![0x00, 0xE8, 0xD8],
                        0x2d => vec![0x78, 0x78, 0x78],
                        0x2e => vec![0x00, 0x00, 0x00],
                        0x2f => vec![0x00, 0x00, 0x00],
                        0x30 => vec![0xFC, 0xFC, 0xFC],
                        0x31 => vec![0xA4, 0xE4, 0xFC],
                        0x32 => vec![0xB8, 0xB8, 0xF8],
                        0x33 => vec![0xD8, 0xB8, 0xF8],
                        0x34 => vec![0xF8, 0xB8, 0xF8],
                        0x35 => vec![0xF8, 0xA4, 0xC0],
                        0x36 => vec![0xF0, 0xD0, 0xB0],
                        0x37 => vec![0xFC, 0xE0, 0xA8],
                        0x38 => vec![0xF8, 0xD8, 0x78],
                        0x39 => vec![0xD8, 0xF8, 0x78],
                        0x3a => vec![0xB8, 0xF8, 0xB8],
                        0x3b => vec![0xB8, 0xF8, 0xD8],
                        0x3c => vec![0x00, 0xFC, 0xFC],
                        0x3d => vec![0xF8, 0xD8, 0xF8],
                        0x3e => vec![0x00, 0x00, 0x00],
                        0x3f => vec![0x00, 0x00, 0x00],
                        _ => vec![0x00, 0x00, 0x00],
                    }
                    .into_iter()
                })
                .collect();

            texture.update(None, &frame, 256 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
        };

        canvas.present();

        if Instant::now() < next_frame_time {
            ::std::thread::sleep(next_frame_time.duration_since(Instant::now()));
        };

        next_frame_time = Instant::now() + Duration::new(0, 1_000_000_000u32 / 60);
    }
}
