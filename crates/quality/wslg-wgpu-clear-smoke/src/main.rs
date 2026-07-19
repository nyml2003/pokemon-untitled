#![forbid(unsafe_code)]

use std::{error::Error, sync::Arc};

use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.055,
    g: 0.071,
    b: 0.094,
    a: 1.0,
};

enum PresentOutcome {
    Presented,
    Reconfigure,
    Skipped,
    ValidationError,
}

struct ClearTarget<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl<'window> ClearTarget<'window> {
    async fn new(window: impl Into<wgpu::SurfaceTarget<'window>>) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(|error| format!("create surface: {error}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("request adapter: {error}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("wslg-wgpu-clear-smoke device"),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("request device: {error}"))?;
        let mut config = surface
            .get_default_config(&adapter, 960, 720)
            .ok_or_else(|| "surface has no supported default configuration".to_owned())?;
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);

        Ok(Self {
            surface,
            device,
            queue,
            config,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    fn present(&mut self) -> PresentOutcome {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.clear(frame);
                return PresentOutcome::Reconfigure;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return PresentOutcome::Reconfigure;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return PresentOutcome::Skipped;
            }
            wgpu::CurrentSurfaceTexture::Validation => return PresentOutcome::ValidationError,
        };
        self.clear(frame);
        PresentOutcome::Presented
    }

    fn clear(&self, frame: wgpu::SurfaceTexture) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wslg-wgpu-clear-smoke encoder"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wslg-wgpu-clear-smoke pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
    }
}

#[derive(Default)]
struct SmokeApp {
    window: Option<Arc<Window>>,
    target: Option<ClearTarget<'static>>,
}

impl SmokeApp {
    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("WSLg WGPU Clear Smoke")
                        .with_inner_size(LogicalSize::new(960.0, 720.0)),
                )
                .map_err(|error| format!("create window: {error}"))?,
        );
        let target = pollster::block_on(ClearTarget::new(window.clone()))?;
        window.request_redraw();
        self.window = Some(window);
        self.target = Some(target);
        Ok(())
    }
}

impl ApplicationHandler for SmokeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("smoke initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window.id() != window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(target) = &mut self.target {
                    target.resize(size);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(target) = &mut self.target else {
                    return;
                };
                match target.present() {
                    PresentOutcome::Presented => window.request_redraw(),
                    PresentOutcome::Reconfigure => {
                        let size = window.inner_size();
                        target.resize(size);
                        window.request_redraw();
                    }
                    PresentOutcome::Skipped => {}
                    PresentOutcome::ValidationError => {
                        eprintln!("smoke presentation failed: WGPU surface validation error");
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = SmokeApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
