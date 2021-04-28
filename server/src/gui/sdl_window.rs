use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use sdl2::{event::Event, keyboard::Keycode};

use super::{EmulationState, NES_HEIGHT, NES_WIDTH};

use super::rgb_value_table::RGB_VALUE_TABLE;

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

    //emulation_state.write().unwrap().is_running = true;
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
                } => controller_state &= !0x80,
                Event::KeyUp {
                    keycode: Some(Keycode::Z),
                    ..
                } => controller_state &= !0x40,
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
                if let Some(frame) = emulation_state.emulator.clock() {
                    break frame;
                }
                /*else if emulation_state.emulator.cpu().pc == 0xf0ca {
                    emulation_state.is_running = false;
                    break &[0u8; 256 * 240];
                }*/
            };

            // Maps 6 bit colors to RGB
            let frame: Vec<u8> = frame
                .iter()
                .flat_map(|c| {
                    RGB_VALUE_TABLE
                        .get(*c as usize)
                        .unwrap_or(&[0x00, 0x00, 0x00])
                })
                .copied()
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
