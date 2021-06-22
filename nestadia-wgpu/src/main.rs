use futures::executor::block_on;
use nestadia::Emulator;
use wgpu::util::DeviceExt;

use std::{
    convert::TryFrom,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
    time::{Duration, Instant},
};

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use bitflags::bitflags;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    rom: Option<PathBuf>,
}

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

// This maps the keyboard input to a controller input
impl TryFrom<&VirtualKeyCode> for ControllerState {
    type Error = ();

    fn try_from(keycode: &VirtualKeyCode) -> Result<Self, ()> {
        match keycode {
            VirtualKeyCode::X => Ok(ControllerState::A),
            VirtualKeyCode::Z => Ok(ControllerState::B),
            VirtualKeyCode::S => Ok(ControllerState::START),
            VirtualKeyCode::A => Ok(ControllerState::SELECT),
            VirtualKeyCode::Down => Ok(ControllerState::DOWN),
            VirtualKeyCode::Left => Ok(ControllerState::LEFT),
            VirtualKeyCode::Right => Ok(ControllerState::RIGHT),
            VirtualKeyCode::Up => Ok(ControllerState::UP),
            _ => Err(()),
        }
    }
}

// Target for NTSC is ~60 FPS
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / 60);

// NES outputs a 256 x 240 pixel image
const NUM_PIXELS: usize = 256 * 240;

// A 2D position is mapped to a 2D texture.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

struct State {
    emulator: Emulator,
    controller1: ControllerState,
    last_frame_time: Instant,

    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,

    screen_texture: wgpu::Texture,
    screen_bind_group: wgpu::BindGroup,
}

impl State {
    /// Create a new state and initialize the rendering pipeline.
    async fn new(window: &winit::window::Window, emulator: Emulator) -> Self {
        let size = window.inner_size();

        // Used prefered graphic API
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Note: Present mode: Immediate is there to disable Vsync since it breaks the timing.
        // We wouldn't have to do this if we were making an actual game, but in the case of a NES emulator,
        // logic is tied to the framerate.
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        // Create the texture to show the emulator screen
        let texture_size = wgpu::Extent3d {
            width: 256,
            height: 240,
            depth_or_array_layers: 1,
        };

        let screen_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Screen Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        // Write an initial black screen before the first frame arrive
        let texture = [0u8; NUM_PIXELS * 4];

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &screen_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &texture,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * 256),
                rows_per_image: std::num::NonZeroU32::new(240),
            },
            texture_size,
        );

        let screen_texture_view =
            screen_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let screen_texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Bind groups are used to access the texture from the shader
        let screen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
            });

        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Bind Group"),
            layout: &screen_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&screen_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&screen_texture_sampler),
                },
            ],
        });

        // Load the shader
        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            flags: wgpu::ShaderFlags::all(),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&screen_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        // Maps the four corner of the screen to the four corner of the texture
        let vertices = [
            Vertex {
                position: [-1.0, -1.0],
                tex_coord: [0.0, 1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
                tex_coord: [0.0, 0.0],
            },
            Vertex {
                position: [1.0, -1.0],
                tex_coord: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                tex_coord: [1.0, 0.0],
            },
        ];

        // Use two triangle to make a square filling the screen.
        let indices: [u16; 6] = [0, 3, 1, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsage::INDEX,
        });

        Self {
            emulator,
            controller1: Default::default(),
            last_frame_time: Instant::now(),

            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,

            screen_texture,
            screen_bind_group,
        }
    }

    /// Update the size of the window so rendering is aware of the change
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    /// This is where we handle controller inputs
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => match input {
                // Handle controller inputs
                KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(key_code),
                    ..
                } => {
                    if let Ok(f) = ControllerState::try_from(key_code) {
                        self.controller1.insert(f);

                        self.emulator.set_controller1(self.controller1.bits());
                        true
                    } else {
                        false
                    }
                }

                KeyboardInput {
                    state: ElementState::Released,
                    virtual_keycode: Some(key_code),
                    ..
                } => {
                    if let Ok(f) = ControllerState::try_from(key_code) {
                        self.controller1.remove(f);

                        self.emulator.set_controller1(self.controller1.bits());
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// Update the game state
    fn update(&mut self) {
        // Clock until a frame is ready
        let frame = loop {
            if let Some(frame) = self.emulator.clock() {
                break frame;
            }
        };

        let mut current_frame = [0u8; NUM_PIXELS * 4];
        nestadia::frame_to_rgba(&frame, &mut current_frame);

        // Update texture
        let texture_size = wgpu::Extent3d {
            width: 256,
            height: 240,
            depth_or_array_layers: 1,
        };

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.screen_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &current_frame,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * 256),
                rows_per_image: std::num::NonZeroU32::new(240),
            },
            texture_size,
        );
    }

    /// Render the screen
    fn render(&mut self) -> Result<(), wgpu::SwapChainError> {
        let frame = self.swap_chain.get_current_frame()?.output;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.screen_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        Ok(())
    }

    fn save_data(&self, save_path: &Path) {
        if let Some(save_data) = self.emulator.get_save_data() {
            if let Ok(mut f) = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&save_path)
            {
                let _ = f.write_all(save_data);
            }
        }
    }
}

fn main() {
    // Parse CLI options
    let opt = Opt::from_args();

    // Create the window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Nestadia")
        .build(&event_loop)
        .unwrap();

    // Find ROM path
    let path = if let Some(p) = opt.rom {
        p
    } else {
        native_dialog::FileDialog::new()
            .add_filter("NES roms", &["nes"])
            .show_open_single_file()
            .unwrap()
            .expect("No rom passed!")
    };

    let mut save_path = path.clone();
    save_path.set_extension("sav");

    // Read the ROM
    let rom = std::fs::read(path).expect("Could not read the ROM file");

    // Read the save file
    let mut save_buf = Vec::new();
    let save_file = if let Ok(mut file) = std::fs::File::open(&save_path) {
        let _ = file.read_to_end(&mut save_buf);
        Some(save_buf.as_slice())
    } else {
        None
    };

    // Create the emulator
    let emulator = Emulator::new(&rom, save_file).expect("Rom parsing failed");

    // Wait until WGPU is ready
    let mut state = block_on(State::new(&window, emulator));

    // Handle window events
    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            state.update();
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SwapChainError::Lost) => state.resize(state.size),
                Err(wgpu::SwapChainError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e),
            }
        }

        // If renderer is free, sync with 60 FPS and request the next frame.
        // Note that this locks FPS at 60, however logic and FPS are bound together on the NES so this is normal.
        Event::RedrawEventsCleared => {
            let elapsed_time = state.last_frame_time.elapsed();
            if elapsed_time >= FRAME_TIME {
                state.last_frame_time = Instant::now();
                window.request_redraw()
            } else {
                *control_flow = ControlFlow::WaitUntil(Instant::now() + FRAME_TIME - elapsed_time)
            }
        }
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => {
            if !state.input(event) {
                match event {
                    // Exit if X button is clicked
                    WindowEvent::CloseRequested => {
                        state.save_data(&save_path);

                        *control_flow = ControlFlow::Exit
                    }

                    // Update rendering if window is resized
                    WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size)
                    }

                    // Exit if ESC is pressed
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => {
                        state.save_data(&save_path);

                        *control_flow = ControlFlow::Exit
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}
