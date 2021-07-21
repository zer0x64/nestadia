#[macro_use]
extern crate bitflags;

use nestadia::Emulator;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};
use yew::{
    prelude::*,
    services::reader::{FileData, ReaderService, ReaderTask},
};
use yew::{virtual_dom::VNode, ChangeData};

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

enum MainMsg {
    /// This is the message that triggers when a ROM is selected
    ChosenRom(ChangeData),

    // This is the callback when the ROM has done loading into the browser
    LoadedRom(FileData),
}

/// Main Component, used to choose the ROM to run.
struct MainComponent {
    emulator_component: VNode,
    link: ComponentLink<Self>,

    reader_tasks: Vec<ReaderTask>,
}

impl Component for MainComponent {
    type Message = MainMsg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Self {
            emulator_component: html! {},
            link,

            reader_tasks: Vec::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            // When the component receive the file, it loads it in memory
            MainMsg::ChosenRom(ChangeData::Files(files)) => {
                if let Some(f) = files.get(0) {
                    self.reader_tasks.push(
                        ReaderService::read_file(f, self.link.callback(MainMsg::LoadedRom))
                            .unwrap(),
                    );
                };

                false
            }

            // When the ROM is loaded, store it in the component
            MainMsg::LoadedRom(f) => {
                self.emulator_component =
                    html! {<EmulatorComponent rom=f.content></EmulatorComponent>};
                true
            }
            _ => false,
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                {self.emulator_component.clone()}
                <input type="file" onchange=self.link.callback(MainMsg::ChosenRom)/>
            </div>
        }
    }
}

/// Main emulator component
struct EmulatorComponent {
    _link: ComponentLink<Self>,
    emulator: Emulator,
    canvas_ref: NodeRef,
    controller1_state: ControllerState,

    _interval_handle: yew::services::interval::IntervalTask,
    _keyup_handle: yew::services::keyboard::KeyListenerHandle,
    _keydown_handle: yew::services::keyboard::KeyListenerHandle,
}

#[derive(Properties, Clone)]
struct RomProps {
    rom: Vec<u8>,
}

enum EmulatorMsg {
    RenderFrame,
    KeyUp(web_sys::KeyboardEvent),
    KeyDown(web_sys::KeyboardEvent),
}

impl Component for EmulatorComponent {
    type Message = EmulatorMsg;
    type Properties = RomProps;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let emulator = Emulator::new(&props.rom, None).unwrap();

        // Render a frame every 1/60th of a second
        let _interval_handle = yew::services::IntervalService::spawn(
            std::time::Duration::from_nanos(1_000_000_000 / 60),
            link.callback(|_| EmulatorMsg::RenderFrame),
        );

        // Handle keypresses
        let window = yew::utils::window();

        let _keyup_handle = yew::services::KeyboardService::register_key_up(
            &window,
            link.callback(EmulatorMsg::KeyUp),
        );
        let _keydown_handle = yew::services::KeyboardService::register_key_down(
            &window,
            link.callback(EmulatorMsg::KeyDown),
        );

        Self {
            _link: link,
            emulator,
            canvas_ref: Default::default(),
            controller1_state: Default::default(),

            _interval_handle,
            _keyup_handle,
            _keydown_handle,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        let mask_reg = self.emulator.get_ppu_mask_reg();

        match msg {
            EmulatorMsg::RenderFrame => {
                // Run until there's a frame
                let frame = loop {
                    if let Some(frame) = self.emulator.clock() {
                        break frame;
                    }
                };

                // Get canvas 2d context
                let context = self
                    .canvas_ref
                    .cast::<HtmlCanvasElement>()
                    .unwrap()
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .unwrap();

                // Convert to RGBA
                let mut rgba_frame = [0u8; 256 * 240 * 4];

                Emulator::frame_to_rgba(mask_reg, &frame, &mut rgba_frame);

                // Draw image data to the canvas
                let image_data =
                    ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgba_frame), 256, 240)
                        .unwrap();

                context.put_image_data(&image_data, 0.0, 0.0).unwrap();

                true
            }
            // Remove the button from the controller state
            EmulatorMsg::KeyUp(e) => {
                let input = match e.key_code() {
                    0x58 => Some(ControllerState::A),
                    0x5a => Some(ControllerState::B),
                    0x41 => Some(ControllerState::SELECT),
                    0x53 => Some(ControllerState::START),
                    0x28 => Some(ControllerState::DOWN),
                    0x25 => Some(ControllerState::LEFT),
                    0x27 => Some(ControllerState::RIGHT),
                    0x26 => Some(ControllerState::UP),
                    _ => None,
                };

                if let Some(f) = input {
                    self.controller1_state.remove(f);

                    self.emulator.set_controller1(self.controller1_state.bits());
                };

                false
            }
            // Add the button from the controller state
            EmulatorMsg::KeyDown(e) => {
                let input = match e.key_code() {
                    0x58 => Some(ControllerState::A),
                    0x5a => Some(ControllerState::B),
                    0x41 => Some(ControllerState::SELECT),
                    0x53 => Some(ControllerState::START),
                    0x28 => Some(ControllerState::DOWN),
                    0x25 => Some(ControllerState::LEFT),
                    0x27 => Some(ControllerState::RIGHT),
                    0x26 => Some(ControllerState::UP),
                    _ => None,
                };

                if let Some(f) = input {
                    self.controller1_state.insert(f);

                    self.emulator.set_controller1(self.controller1_state.bits());
                };

                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        // If the rom changes, reload the emulator.
        self.emulator = Emulator::new(&props.rom, None).unwrap();
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <canvas width=256 height=240 ref=self.canvas_ref.clone()></canvas>
            </div>
        }
    }
}

fn main() {
    yew::start_app::<MainComponent>();
}
