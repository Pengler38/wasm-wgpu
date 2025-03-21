use std::sync::Arc;
use pollster;

use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}
};

use wgpu::util::DeviceExt;

mod platform_specific;
mod letters;

struct Model {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    models: Vec<Model>,
    letter_bind_group: wgpu::BindGroup,
}

impl State {
    async fn new(window: Arc<Window>, content: &Content) -> State {

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

        // Handle the letter texture
        let texture_size = wgpu::Extent3d {
            width: content.letter_texture.width,
            height: content.letter_texture.height,
            depth_or_array_layers: 1,
        };

        let letter_texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("letter_texture"),
                view_formats: &[],
            }
        );

        queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &letter_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(content.letter_texture.rgba_values.as_slice()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * content.letter_texture.width),
                rows_per_image: Some(content.letter_texture.height),
            },
            texture_size,
        );

        let letter_texture_view = letter_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let letter_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let letter_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("texture_bind_group_layout"),
        });

        let letter_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &letter_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&letter_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&letter_sampler),
                    },
                ],
                label: Some("letter_bind_group"),
            }
        );


        //Create the Render Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&letter_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    letters::desc()
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

        let mut models: Vec<Model> = vec![];
        for letter in &content.alphabet_models {
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

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            models,
            letter_bind_group,
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

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        //Reconfigure the surface
        self.configure_surface();
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
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0, }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Draw commands
        renderpass.set_pipeline(&self.render_pipeline);
        renderpass.set_bind_group(0, &self.letter_bind_group, &[]);
        renderpass.set_vertex_buffer(0, self.models[0].vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.models[0].index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        
        renderpass.draw_indexed(0..self.models[0].num_indices, 0, 0..1);

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
    content: Content,
}

#[derive(Clone)]
struct Content {
    alphabet_models: Vec<letters::Model>,
    text: String,
    letter_texture: letters::Texture,
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

        let state = pollster::block_on(State::new(window.clone(), &self.content));
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
        content: Content {
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
        //Spawn_app is similar to run_app, but preferred for wasm since it does not require using
        //exceptions for control flow and cluttering the web console
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app); 
    }
    Ok(())
}
