use cgmath::prelude::*;

use std::sync::Arc;
use pollster;

use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}
};

use wgpu::util::DeviceExt;

mod platform_specific;
mod letters;
mod texture;

struct Model {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    scale: f32,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: ( cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation) * cgmath::Matrix4::from_scale(self.scale) ).into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // Steps on each change of the instance, not the vertex
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBS
        }
    }
}

struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    fn new_default(aspect_ratio: f32) -> Self {
        Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: aspect_ratio,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        }
    }

    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

struct State {
    camera: Camera,
    camera_uniform: CameraUniform,

    //wgpu oriented portion of state
    camera_buffer: wgpu::Buffer,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    models: Vec<Model>,
    instances: [Vec<Instance>; 26],
    instance_buffers: [wgpu::Buffer; 26],
    bind_groups: Vec<wgpu::BindGroup>,
}

impl State {
    async fn new(window: Arc<Window>, init_content: &InitContent) -> State {

        // Handle wgpu portion of State creation:
        let instance_descriptor = platform_specific::instance_descriptor();
        let instance = wgpu::Instance::new(&instance_descriptor);

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter_options = wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        };

        let adapter = instance
            .request_adapter(&adapter_options)
            .await
            .unwrap();

        let device_descriptor = platform_specific::device_descriptor();
        let (device, queue) = adapter
            .request_device(&device_descriptor, None)
            .await
            .unwrap();

        let size = window.inner_size(); //This is zero on wasm during init and causes errors
                                        //if you configure the surface with a size of zero

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(cap.formats[0]);

        // Start populating the bind_groups
        let mut bind_groups = vec![];
        let mut bind_group_layouts = vec![];

        // Load the letter texture into the gpu
        let letter_texture = texture::GpuTexture::from_rgbatexture( &init_content.letter_texture, &device, &queue, "letter_texture" );

        // Create the bind group
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("bind_group_layout"),
        });

        let texture_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&letter_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&letter_texture.sampler),
                    },
                ],
                label: Some("bind_group"),
            }
        );
        bind_group_layouts.push(&texture_bind_group_layout);
        bind_groups.push(texture_bind_group);

        // Camera initialization
        let camera = Camera::new_default(size.width as f32 / size.height as f32);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("camera_buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        bind_group_layouts.push(&camera_bind_group_layout);
        bind_groups.push(camera_bind_group);

        //Create the Render Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: bind_group_layouts.as_slice(),
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    letters::desc(),
                    InstanceRaw::desc(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires
                // Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requres Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requres Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Load the alphabet models into buffers
        let mut models: Vec<Model> = vec![];
        for letter in &init_content.alphabet_models {
            let model = Model {
                vertex_buffer: device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor{
                        label: Some("vertex_buffer"),
                        contents: bytemuck::cast_slice(&letter.verts),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                index_buffer: device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor{
                        label: Some("index_buffer"),
                        contents: bytemuck::cast_slice(&letter.tri_idxs),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                num_indices: letter.number_indices(),
            };
            models.push(model);
        }

        // Get the required instances from the text display
        let instances: [Vec<Instance>; 26] = get_letter_instances(&init_content.text);
        let instance_data: [Vec<InstanceRaw>; 26] = instances.iter().map(
            |instances| instances.iter().map(
                |instance| instance.to_raw()
            ).collect::<Vec<InstanceRaw>>()
        ).collect::<Vec<_>>().try_into().unwrap();

        let instance_buffers: [wgpu::Buffer; 26] = instance_data.iter().enumerate().map(
            |(i, v)| device.create_buffer_init( &wgpu::util::BufferInitDescriptor {
                label: Some(&("instance_buffer index: ".to_string() + &i.to_string())),
                contents: bytemuck::cast_slice(&v),
                usage: wgpu::BufferUsages::VERTEX,
            })
        ).collect::<Vec<_>>().try_into().unwrap();

        let state = State {
            camera,
            camera_uniform,
            camera_buffer,
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            models,
            instances,
            instance_buffers,
            bind_groups,
        };

        //Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        //If size is zero, do not reconfigure surface. Causes wgpu errors
        if self.size.width == 0 || self.size.height == 0 { 
            return;
        }

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            //Request compatibility with the sRGB-format texture view we're going to create later
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn reconfigure_camera(&mut self) {
        self.camera = Camera::new_default( self.size.width as f32 / self.size.height as f32);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        //Reconfigure the surface
        self.configure_surface();
        self.reconfigure_camera();
    }

    fn render(&mut self) {
        //Create texture view
        let output = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swapchain texture");
        let output_texture_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                //Without add_srgb_suffix the image we will be working with might not be "gamma
                //correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        //Renders the content
        let mut encoder = self.device.create_command_encoder(&Default::default());
        //Create the render pass which will clear the screen
        let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0, }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw commands
        renderpass.set_pipeline(&self.render_pipeline);
        for (i, bind_group) in self.bind_groups.iter().enumerate() {
            renderpass.set_bind_group(i as u32, bind_group, &[]);
        }

        // Draw each letter
        for letter in 0..26 {
            if self.instances[letter].len() > 0 {
                renderpass.set_vertex_buffer(0, self.models[letter].vertex_buffer.slice(..));
                renderpass.set_vertex_buffer(1, self.instance_buffers[letter].slice(..));
                renderpass.set_index_buffer(self.models[letter].index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                renderpass.draw_indexed(0..self.models[letter].num_indices, 0, 0..self.instances[letter].len() as u32);
            }
        }

        //End the render pass, releasing the borrow of encoder
        drop(renderpass);

        //Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        output.present();
    }
}

struct App {
    state: Option<State>,
    init_content: InitContent,
}

// InitContent includes (effectively static) content generated during initialization
struct InitContent {
    alphabet_models: Vec<letters::Model>,
    text: String,
    letter_texture: texture::RgbaTexture,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        //Create window object
        let window_attributes = platform_specific::window_attributes();
        let window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window.clone(), &self.init_content));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("Closing window...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                //Emit a new redraw requested event
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                //Reconfigures the size of the surface.
                //No re-render required, this event is always followed by a redraw request
                state.resize(size);
            }
            _ => (),
        }
    }
}


// Translates a string into the equivalent instances to render the correct letters at the right locations
// Currently does only one line and only handles lowercase letters
// Instances will be from x=[-5, 5], at z=???. Each letter will be scaled down in height to match the width
fn get_letter_instances(text: &str) -> [Vec<Instance>; 26] {
    const LEFT_BOUND: f32 = -5.0;
    const RIGHT_BOUND: f32 = 5.0;
    let length = f32::abs(LEFT_BOUND) + f32::abs(RIGHT_BOUND);
    let mut letter_instances: [Vec<Instance>; 26] = std::array::from_fn(|_| Vec::new());
    let num_characters = text.len();

    for (i, char) in text.chars().enumerate() {
        let width_per_character = length / num_characters as f32;
        let scale = width_per_character * 0.75;
        let x = LEFT_BOUND
            + (i as f32 + 0.5) * width_per_character;
        let position = cgmath::Vector3 { x, y: 0.0, z: -5.0 };
        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
        };

        let idx = letter_index(char);
        letter_instances[idx].push( Instance {
            position, rotation, scale
        });
    }

    letter_instances
}

fn letter_index(c: char) -> usize {
    if c.is_ascii() {
        c.to_ascii_lowercase() as usize - 97
    } else {
        panic!("Character passed in was not ascii!");
    }
}

fn main() -> Result<(), winit::error::EventLoopError>{
    //Set up wgpu panic hook
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once(); //This should be done on init once

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let alphabet_models = letters::create_alphabet_models();
    let text = "hello".to_string();
    let letter_texture = letters::create_letter_texture();

    #[allow(unused_mut)] // mut used in desktop and not in wasm32
    let mut app = App {
        state: None,
        init_content: InitContent {
            alphabet_models,
            text,
            letter_texture,
        },
    };
        
    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wgpu::web_sys;
        use web_sys::wasm_bindgen::JsCast;

        let linecount_heading = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("linecount_heading"))
            .map(|e| e.dyn_into::<web_sys::HtmlElement>().unwrap())
            .unwrap();
        let s = linecount_heading.inner_text().replace(
            "____",
            include!(concat!(env!("OUT_DIR"), "/linecount.txt")) );
        // Utilizes the linecount.txt file generated by the build.rs script
        linecount_heading.set_inner_text(&s);

        //Spawn_app is similar to run_app, but preferred for wasm since it does not require using
        //exceptions for control flow and cluttering the web console
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app); 
    }
    Ok(())
}
