//! Gen3's single native GPU submission boundary.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, PrepareError, RenderError,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use punctum_gpu::{PixelSize, Rgba8, Viewport as GridViewport};
use punctum_wgpu::{GpuRuntime, GpuRuntimeError};

pub use game_native_plan::{
    FramePlan, FramePlanError, NativeAssetError, NativeAssets, NativeTextBounds, NativeTextLabel,
    TextScale, text_bounds,
};
pub use punctum_wgpu::{
    PresentOutcome, WinitCommittedTextSnapshot, WinitKeyEventSnapshot, normalize_committed_text,
    normalize_key_event,
};

pub struct NativeTarget<'window> {
    runtime: GpuRuntime<'window>,
    text: NativeTextRenderer,
}

impl<'window> NativeTarget<'window> {
    pub fn new(
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        surface_size: PixelSize,
        assets: &NativeAssets,
        clear_color: Rgba8,
    ) -> Result<Self, NativeTargetError> {
        let instance = wgpu::Instance::default();
        let runtime = pollster::block_on(GpuRuntime::new(
            &instance,
            target,
            surface_size,
            assets.atlas(),
            clear_color,
        ))?;
        Ok(Self {
            runtime,
            text: NativeTextRenderer::new(),
        })
    }

    pub const fn surface_size(&self) -> PixelSize {
        self.runtime.surface_size()
    }

    pub fn resize(&mut self, size: PixelSize) {
        self.runtime.resize(size);
    }

    pub fn present(&mut self, frame: &FramePlan) -> Result<PresentOutcome, NativeTargetError> {
        if frame.labels().is_empty() {
            return self
                .runtime
                .present_plan(frame.gpu())
                .map_err(NativeTargetError::Gpu);
        }

        let mut text_result = Ok(());
        let text = &mut self.text;
        let result = self.runtime.present_plan_with_overlay(
            frame.gpu(),
            |device, queue, target, encoder, format, surface_size| {
                text_result = text.encode(
                    frame.labels(),
                    frame.viewport(),
                    frame.text_scale(),
                    device,
                    queue,
                    target,
                    encoder,
                    format,
                    surface_size,
                );
            },
        );
        text_result?;
        result.map_err(NativeTargetError::Gpu)
    }
}

struct NativeTextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    gpu: Option<TextGpu>,
}

impl NativeTextRenderer {
    fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            gpu: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        &mut self,
        labels: &[NativeTextLabel],
        viewport: GridViewport,
        text_scale: TextScale,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        format: wgpu::TextureFormat,
        surface_size: PixelSize,
    ) -> Result<(), TextRenderError> {
        if self.gpu.as_ref().is_none_or(|gpu| gpu.format != format) {
            self.gpu = Some(TextGpu::new(device, queue, format));
        }
        self.gpu
            .as_mut()
            .expect("text GPU resources were initialized")
            .encode(
                labels,
                viewport,
                text_scale,
                device,
                queue,
                target,
                encoder,
                surface_size,
                &mut self.font_system,
                &mut self.swash_cache,
            )
    }
}

struct TextGpu {
    format: wgpu::TextureFormat,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
}

impl TextGpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self {
            format,
            viewport,
            atlas,
            renderer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        &mut self,
        labels: &[NativeTextLabel],
        grid_viewport: GridViewport,
        text_scale: TextScale,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        surface_size: PixelSize,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
    ) -> Result<(), TextRenderError> {
        self.viewport.update(
            queue,
            Resolution {
                width: surface_size.width,
                height: surface_size.height,
            },
        );

        let mut buffers = Vec::with_capacity(labels.len());
        let mut areas = Vec::with_capacity(labels.len());
        for label in labels {
            let bounds = text_bounds(label, grid_viewport)
                .map_err(|_| TextRenderError::CoordinateOverflow)?;
            let mut buffer = Buffer::new(
                font_system,
                Metrics::new(
                    text_scale.font_size(grid_viewport.cell_size.height),
                    bounds.height().max(1) as f32,
                ),
            );
            buffer.set_size(
                Some(bounds.width().max(1) as f32),
                Some(bounds.height().max(1) as f32),
            );
            buffer.set_text(
                &label.content,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);
            buffers.push(buffer);
            areas.push((
                bounds,
                Color::rgba(
                    label.color.red,
                    label.color.green,
                    label.color.blue,
                    label.color.alpha,
                ),
            ));
        }

        self.renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            buffers
                .iter()
                .zip(&areas)
                .map(|(buffer, (bounds, color))| TextArea {
                    buffer,
                    left: bounds.left as f32,
                    top: bounds.top as f32,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: bounds.left,
                        top: bounds.top,
                        right: bounds.right,
                        bottom: bounds.bottom,
                    },
                    default_color: *color,
                    custom_glyphs: &[],
                }),
            swash_cache,
        )?;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gen3 native text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        self.renderer
            .render(&self.atlas, &self.viewport, &mut pass)?;
        drop(pass);
        self.atlas.trim();
        Ok(())
    }
}

#[derive(Debug)]
pub enum NativeTargetError {
    Gpu(GpuRuntimeError),
    Text(TextRenderError),
}

impl fmt::Display for NativeTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gpu(error) => write!(formatter, "native GPU submission failed: {error}"),
            Self::Text(error) => write!(formatter, "native text submission failed: {error}"),
        }
    }
}

impl Error for NativeTargetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Gpu(error) => Some(error),
            Self::Text(error) => Some(error),
        }
    }
}

impl From<GpuRuntimeError> for NativeTargetError {
    fn from(error: GpuRuntimeError) -> Self {
        Self::Gpu(error)
    }
}

impl From<TextRenderError> for NativeTargetError {
    fn from(error: TextRenderError) -> Self {
        Self::Text(error)
    }
}

#[derive(Debug)]
pub enum TextRenderError {
    CoordinateOverflow,
    Prepare(PrepareError),
    Render(RenderError),
}

impl fmt::Display for TextRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateOverflow => formatter.write_str("native text coordinates overflowed"),
            Self::Prepare(error) => write!(formatter, "failed to prepare native text: {error}"),
            Self::Render(error) => write!(formatter, "failed to render native text: {error}"),
        }
    }
}

impl Error for TextRenderError {}

impl From<PrepareError> for TextRenderError {
    fn from(error: PrepareError) -> Self {
        Self::Prepare(error)
    }
}

impl From<RenderError> for TextRenderError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}
