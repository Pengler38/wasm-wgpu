use std::sync::Arc;
use pollster;

use winit::{
    application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}
};

use wgpu::util::DeviceExt;

mod platform_specific;
mod letters;

struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
}

impl State {
    async fn new(window: Arc<Window>) -> State {

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


        //Create the Render Pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
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

        let models = letters::create_alphabet_models();

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("Vertex buffer"),
                contents: bytemuck::cast_slice(&models[0].verts),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor{
                label: Some("Index buffer"),
                contents: bytemuck::cast_slice(&models[0].tri_idxs),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: models[0].tri_idxs.len() as u32 * 3,
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

        //Renders a green screen
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
        renderpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        renderpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        renderpass.draw_indexed(0..self.num_indices, 0, 0..1);

        //End the render pass, releasing the borrow of encoder
        drop(renderpass);

        //Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        output.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
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


        let state = pollster::block_on(State::new(window.clone()));
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::default();
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::default();

        //Spawn_app is similar to run_app, but preferred for wasm since it does not require using
        //exceptions for control flow and cluttering the web console
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app); 
    }
    Ok(())
}
