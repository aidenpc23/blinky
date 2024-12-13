use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use wgpu::Color;
use winit::{
    dpi::PhysicalSize,
    event::WindowEvent,
    window::{Window, WindowAttributes},
};

pub struct Client<'a> {
    window: Arc<Window>,
    exit: bool,
    prev_update: Instant,
    frame_target: Instant,
    frame_time: Duration,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    encoder: wgpu::CommandEncoder,
    config: wgpu::SurfaceConfiguration,
    color: u32,
}

impl Client<'_> {
    pub fn new(event_loop: &winit::event_loop::ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .expect("Failed to create window"),
        );

        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("Could not create window surface!");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("Could not get adapter!");

        let buf_size = (10f32.powi(9) * 1.5) as u32;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::PUSH_CONSTANTS
                    | wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
                required_limits: wgpu::Limits {
                    max_storage_buffer_binding_size: buf_size,
                    max_buffer_size: buf_size as u64,
                    max_push_constant_size: 4,
                    ..Default::default()
                },
                memory_hints: wgpu::MemoryHints::default(),
            },
            None, // Trace path
        ))
        .expect("Could not get device!");

        // TODO: use a logger
        let info = adapter.get_info();
        println!("Adapter: {}", info.name);
        println!("Backend: {:?}", info.backend);

        let surface_caps = surface.get_capabilities(&adapter);
        // Set surface format to srbg
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        // create surface config
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        Self {
            window,
            exit: false,
            prev_update: Instant::now(),
            frame_target: Instant::now(),
            frame_time: Duration::from_secs_f32(1.0 / 60.0),
            encoder: Self::create_encoder(&device),
            device,
            queue,
            surface,
            config,
            color: 0,
        }
    }

    fn create_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        })
    }

    pub fn update(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.prev_update;
        self.prev_update = now;

        if self.exit {
            event_loop.exit();
        }
    }

    pub fn draw(&mut self) {
        let mut encoder = std::mem::replace(&mut self.encoder, Self::create_encoder(&self.device));
        let output = self.surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let c = match self.color {
            0 => Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 },
            1 => Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
            2 => Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 },
            _ => Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 },
        };

        self.color = (self.color + 1) % 3;

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(c),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    pub fn window_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::RedrawRequested => {
                self.draw();
                self.window.request_redraw();
            }
            _ => (),
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }
}
