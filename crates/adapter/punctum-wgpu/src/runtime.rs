use std::{error::Error, fmt};

use punctum_gpu::{
    GpuAtlas, GpuCell, GpuClip, GpuPlanError, INSTANCE_STRIDE, PixelSize, Rgba8, SubmissionMode,
    SubmissionPlan, UNIFORM_SIZE, Viewport, encode_instances, encode_uniform, plan_patch,
    plan_surface,
};
use punctum_grid::{GridSize, Patch, Surface};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentOutcome {
    Presented,
    PresentedAndReconfigured,
    SkippedMinimized,
    SkippedTimeout,
    SkippedOccluded,
    Reconfigured,
    SurfaceLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceAcquisition {
    Success,
    Suboptimal,
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceFramePolicy {
    encode_overlay: bool,
    reconfigure: bool,
    outcome: Option<PresentOutcome>,
}

impl SurfaceFramePolicy {
    const fn encode(outcome: PresentOutcome, reconfigure: bool) -> Self {
        Self {
            encode_overlay: true,
            reconfigure,
            outcome: Some(outcome),
        }
    }

    const fn skip(outcome: PresentOutcome, reconfigure: bool) -> Self {
        Self {
            encode_overlay: false,
            reconfigure,
            outcome: Some(outcome),
        }
    }

    const fn validation_error() -> Self {
        Self {
            encode_overlay: false,
            reconfigure: false,
            outcome: None,
        }
    }
}

const fn surface_frame_policy(acquisition: SurfaceAcquisition) -> SurfaceFramePolicy {
    match acquisition {
        SurfaceAcquisition::Success => SurfaceFramePolicy::encode(PresentOutcome::Presented, false),
        SurfaceAcquisition::Suboptimal => {
            SurfaceFramePolicy::encode(PresentOutcome::PresentedAndReconfigured, true)
        }
        SurfaceAcquisition::Timeout => {
            SurfaceFramePolicy::skip(PresentOutcome::SkippedTimeout, false)
        }
        SurfaceAcquisition::Occluded => {
            SurfaceFramePolicy::skip(PresentOutcome::SkippedOccluded, false)
        }
        SurfaceAcquisition::Outdated => {
            SurfaceFramePolicy::skip(PresentOutcome::Reconfigured, true)
        }
        SurfaceAcquisition::Lost => SurfaceFramePolicy::skip(PresentOutcome::SurfaceLost, false),
        SurfaceAcquisition::Validation => SurfaceFramePolicy::validation_error(),
    }
}

pub struct GpuRuntime<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    surface_size: PixelSize,
    configured: bool,
    atlas_size: PixelSize,
    max_instances: u32,
    instance_buffer: wgpu::Buffer,
    instance_capacity: u32,
    grid_size: Option<GridSize>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    clear_color: wgpu::Color,
}

impl<'window> GpuRuntime<'window> {
    pub async fn new(
        instance: &wgpu::Instance,
        target: impl Into<wgpu::SurfaceTarget<'window>>,
        surface_size: PixelSize,
        atlas: &GpuAtlas,
        clear_color: Rgba8,
    ) -> Result<Self, GpuRuntimeError> {
        let surface = instance
            .create_surface(target)
            .map_err(GpuRuntimeError::CreateSurface)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(GpuRuntimeError::RequestAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("punctum-gpu device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(GpuRuntimeError::RequestDevice)?;

        let width = surface_size.width.max(1);
        let height = surface_size.height.max(1);
        let mut config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(GpuRuntimeError::UnsupportedSurface)?;
        config.width = width;
        config.height = height;

        let configured = !surface_size.is_empty();
        if configured {
            surface.configure(&device, &config);
        }

        let atlas_texture = create_atlas_texture(&device, &queue, atlas);
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("punctum-gpu nearest sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("punctum-gpu viewport uniform"),
            size: UNIFORM_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = create_bind_group_layout(&device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("punctum-gpu bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline = create_pipeline(&device, config.format, &bind_group_layout);
        let instance_buffer = create_instance_buffer(&device, 1);
        let max_instances =
            (device.limits().max_buffer_size / INSTANCE_STRIDE).min(u64::from(u32::MAX)) as u32;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            surface_size,
            configured,
            atlas_size: atlas.size(),
            max_instances,
            instance_buffer,
            instance_capacity: 1,
            grid_size: None,
            uniform_buffer,
            bind_group,
            pipeline,
            clear_color: color_to_wgpu(clear_color),
        })
    }

    pub const fn surface_size(&self) -> PixelSize {
        self.surface_size
    }

    pub fn resize(&mut self, size: PixelSize) {
        self.surface_size = size;
        if size.is_empty() {
            self.configured = false;
            return;
        }

        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        self.configured = true;
    }

    pub fn present_surface(
        &mut self,
        surface: &Surface<GpuCell>,
        atlas: &GpuAtlas,
        viewport: Viewport,
        clip: GpuClip,
    ) -> Result<PresentOutcome, GpuRuntimeError> {
        let plan = plan_surface(surface, atlas, self.max_instances, viewport, clip)?;
        self.present_plan(&plan)
    }

    pub fn present_patch(
        &mut self,
        patch: &Patch<GpuCell>,
        atlas: &GpuAtlas,
        viewport: Viewport,
        clip: GpuClip,
    ) -> Result<PresentOutcome, GpuRuntimeError> {
        let plan = plan_patch(patch, atlas, self.max_instances, viewport, clip)?;
        self.present_plan(&plan)
    }

    pub fn present_plan(
        &mut self,
        plan: &SubmissionPlan,
    ) -> Result<PresentOutcome, GpuRuntimeError> {
        self.present_plan_with_overlay(plan, |_, _, _, _, _, _| {})
    }

    pub fn present_plan_with_overlay<F>(
        &mut self,
        plan: &SubmissionPlan,
        encode_overlay: F,
    ) -> Result<PresentOutcome, GpuRuntimeError>
    where
        F: FnOnce(
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
            &mut wgpu::CommandEncoder,
            wgpu::TextureFormat,
            PixelSize,
        ),
    {
        let mut overlay = Some(encode_overlay);
        self.present_plans_with_overlays(&[plan], move |_, device, queue, target, encoder, format, size| {
            overlay
                .take()
                .expect("the single-plan overlay is encoded once")(
                device, queue, target, encoder, format, size,
            );
        })
    }

    pub fn present_plans_with_overlays<F>(
        &mut self,
        plans: &[&SubmissionPlan],
        mut encode_overlay: F,
    ) -> Result<PresentOutcome, GpuRuntimeError>
    where
        F: FnMut(
            usize,
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
            &mut wgpu::CommandEncoder,
            wgpu::TextureFormat,
            PixelSize,
        ),
    {
        for plan in plans {
            if plan.viewport.target_size != self.surface_size {
                return Err(GpuRuntimeError::ViewportSizeMismatch {
                    viewport_size: plan.viewport.target_size,
                    surface_size: self.surface_size,
                });
            }
        }
        if !self.configured || self.surface_size.is_empty() {
            return Ok(PresentOutcome::SkippedMinimized);
        }

        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                self.render_plans_with_overlays(frame, plans, &mut encode_overlay)?;
                self.finish_acquired_frame(SurfaceAcquisition::Success)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.render_plans_with_overlays(frame, plans, &mut encode_overlay)?;
                self.finish_acquired_frame(SurfaceAcquisition::Suboptimal)
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.finish_unacquired_frame(SurfaceAcquisition::Timeout)
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.finish_unacquired_frame(SurfaceAcquisition::Occluded)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.finish_unacquired_frame(SurfaceAcquisition::Outdated)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                self.finish_unacquired_frame(SurfaceAcquisition::Lost)
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.finish_unacquired_frame(SurfaceAcquisition::Validation)
            }
        }
    }

    fn finish_acquired_frame(
        &self,
        acquisition: SurfaceAcquisition,
    ) -> Result<PresentOutcome, GpuRuntimeError> {
        let policy = surface_frame_policy(acquisition);
        debug_assert!(policy.encode_overlay);
        if policy.reconfigure {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(policy.outcome.expect("acquired frames have an outcome"))
    }

    fn finish_unacquired_frame(
        &self,
        acquisition: SurfaceAcquisition,
    ) -> Result<PresentOutcome, GpuRuntimeError> {
        let policy = surface_frame_policy(acquisition);
        debug_assert!(!policy.encode_overlay);
        if policy.reconfigure {
            self.surface.configure(&self.device, &self.config);
        }
        policy.outcome.ok_or(GpuRuntimeError::SurfaceValidation)
    }

    fn apply_uploads(&mut self, plan: &SubmissionPlan) -> Result<(), GpuRuntimeError> {
        match plan.mode {
            SubmissionMode::Replace => {
                self.ensure_instance_capacity(plan.instance_count)?;
                self.grid_size = Some(plan.grid_size);
            }
            SubmissionMode::Delta => {
                if self.grid_size != Some(plan.grid_size) {
                    return Err(GpuRuntimeError::DeltaGridMismatch {
                        current: self.grid_size,
                        patch: plan.grid_size,
                    });
                }
            }
        }

        for upload in &plan.uploads {
            let offset = u64::from(upload.first_slot) * INSTANCE_STRIDE;
            self.queue.write_buffer(
                &self.instance_buffer,
                offset,
                &encode_instances(&upload.instances),
            );
        }
        Ok(())
    }

    fn ensure_instance_capacity(&mut self, count: u32) -> Result<(), GpuRuntimeError> {
        if count <= self.instance_capacity {
            return Ok(());
        }

        let required = u64::from(count) * INSTANCE_STRIDE;
        let maximum = self.device.limits().max_buffer_size;
        if required > maximum {
            return Err(GpuRuntimeError::InstanceBufferTooLarge { required, maximum });
        }

        self.instance_buffer = create_instance_buffer(&self.device, count);
        self.instance_capacity = count;
        Ok(())
    }

    fn render_plans_with_overlays<F>(
        &mut self,
        frame: wgpu::SurfaceTexture,
        plans: &[&SubmissionPlan],
        encode_overlay: &mut F,
    ) -> Result<(), GpuRuntimeError>
    where
        F: FnMut(
            usize,
            &wgpu::Device,
            &wgpu::Queue,
            &wgpu::TextureView,
            &mut wgpu::CommandEncoder,
            wgpu::TextureFormat,
            PixelSize,
        ),
    {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        for (index, plan) in plans.iter().enumerate() {
            self.apply_uploads(plan)?;
            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                &encode_uniform(plan.viewport, self.atlas_size),
            );
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("punctum-gpu frame encoder"),
                });
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("punctum-gpu render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if index == 0 {
                                wgpu::LoadOp::Clear(self.clear_color)
                            } else {
                                wgpu::LoadOp::Load
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                if let Some(scissor) = plan.scissor {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                    pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
                    pass.draw(0..6, 0..plan.instance_count);
                }
            }
            encode_overlay(
                index,
                &self.device,
                &self.queue,
                &view,
                &mut encoder,
                self.config.format,
                self.surface_size,
            );
            self.queue.submit([encoder.finish()]);
        }
        self.queue.present(frame);
        Ok(())
    }
}

fn create_atlas_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: &GpuAtlas,
) -> wgpu::Texture {
    let size = atlas.size();
    let extent = wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("punctum-gpu atlas"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        atlas.rgba8(),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 4),
            rows_per_image: Some(size.height),
        },
        extent,
    );
    texture
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("punctum-gpu bind group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(UNIFORM_SIZE),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("punctum-gpu shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("punctum-gpu pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    let attributes = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32x2,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32x2,
            offset: 8,
            shader_location: 1,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Sint32x2,
            offset: 16,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32x4,
            offset: 24,
            shader_location: 3,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Unorm8x4,
            offset: 40,
            shader_location: 4,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32,
            offset: 44,
            shader_location: 5,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32x4,
            offset: 48,
            shader_location: 6,
        },
    ];
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: INSTANCE_STRIDE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("punctum-gpu render pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &buffers,
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_instance_buffer(device: &wgpu::Device, count: u32) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("punctum-gpu instance buffer"),
        size: u64::from(count.max(1)) * INSTANCE_STRIDE,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn color_to_wgpu(color: Rgba8) -> wgpu::Color {
    const SCALE: f64 = 1.0 / 255.0;
    wgpu::Color {
        r: f64::from(color.red) * SCALE,
        g: f64::from(color.green) * SCALE,
        b: f64::from(color.blue) * SCALE,
        a: f64::from(color.alpha) * SCALE,
    }
}

#[derive(Debug)]
pub enum GpuRuntimeError {
    CreateSurface(wgpu::CreateSurfaceError),
    RequestAdapter(wgpu::RequestAdapterError),
    RequestDevice(wgpu::RequestDeviceError),
    UnsupportedSurface,
    Plan(GpuPlanError),
    ViewportSizeMismatch {
        viewport_size: PixelSize,
        surface_size: PixelSize,
    },
    DeltaGridMismatch {
        current: Option<GridSize>,
        patch: GridSize,
    },
    InstanceBufferTooLarge {
        required: u64,
        maximum: u64,
    },
    SurfaceValidation,
}

impl fmt::Display for GpuRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateSurface(error) => {
                write!(formatter, "failed to create GPU surface: {error}")
            }
            Self::RequestAdapter(error) => {
                write!(formatter, "failed to request GPU adapter: {error}")
            }
            Self::RequestDevice(error) => {
                write!(formatter, "failed to request GPU device: {error}")
            }
            Self::UnsupportedSurface => {
                formatter.write_str("GPU surface has no supported configuration")
            }
            Self::Plan(error) => write!(formatter, "GPU submission planning failed: {error}"),
            Self::ViewportSizeMismatch {
                viewport_size,
                surface_size,
            } => write!(
                formatter,
                "viewport target {viewport_size:?} does not match surface {surface_size:?}"
            ),
            Self::DeltaGridMismatch { current, patch } => write!(
                formatter,
                "delta patch grid {patch:?} does not match current GPU grid {current:?}"
            ),
            Self::InstanceBufferTooLarge { required, maximum } => write!(
                formatter,
                "GPU instance buffer requires {required} bytes, device supports {maximum}"
            ),
            Self::SurfaceValidation => {
                formatter.write_str("GPU surface acquisition validation failed")
            }
        }
    }
}

impl Error for GpuRuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateSurface(error) => Some(error),
            Self::RequestAdapter(error) => Some(error),
            Self::RequestDevice(error) => Some(error),
            Self::Plan(error) => Some(error),
            _ => None,
        }
    }
}

impl From<GpuPlanError> for GpuRuntimeError {
    fn from(error: GpuPlanError) -> Self {
        Self::Plan(error)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
        thread,
    };

    use punctum_gpu::{GpuResource, PixelRect, ResourceId};

    use super::*;

    struct ThreadWake(thread::Thread);

    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => thread::park(),
            }
        }
    }

    #[test]
    fn surface_frame_policy_controls_overlay_and_outcome() {
        let cases = [
            (
                SurfaceAcquisition::Success,
                SurfaceFramePolicy::encode(PresentOutcome::Presented, false),
            ),
            (
                SurfaceAcquisition::Suboptimal,
                SurfaceFramePolicy::encode(PresentOutcome::PresentedAndReconfigured, true),
            ),
            (
                SurfaceAcquisition::Timeout,
                SurfaceFramePolicy::skip(PresentOutcome::SkippedTimeout, false),
            ),
            (
                SurfaceAcquisition::Occluded,
                SurfaceFramePolicy::skip(PresentOutcome::SkippedOccluded, false),
            ),
            (
                SurfaceAcquisition::Outdated,
                SurfaceFramePolicy::skip(PresentOutcome::Reconfigured, true),
            ),
            (
                SurfaceAcquisition::Lost,
                SurfaceFramePolicy::skip(PresentOutcome::SurfaceLost, false),
            ),
            (
                SurfaceAcquisition::Validation,
                SurfaceFramePolicy::validation_error(),
            ),
        ];

        for (acquisition, expected) in cases {
            assert_eq!(surface_frame_policy(acquisition), expected);
        }
    }

    #[test]
    #[ignore = "requires a local GPU adapter"]
    fn headless_pipeline_smoke() {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("local GPU adapter");
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("punctum-gpu smoke device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("local GPU device");
        let layout = create_bind_group_layout(&device);
        let pipeline = create_pipeline(&device, wgpu::TextureFormat::Rgba8Unorm, &layout);
        let atlas = GpuAtlas::new(
            PixelSize::new(1, 1),
            vec![255, 255, 255, 255],
            &[GpuResource::new(ResourceId(1), PixelRect::new(0, 0, 1, 1))],
        )
        .unwrap();
        let texture = create_atlas_texture(&device, &queue, &atlas);

        drop((pipeline, texture));
    }
}
