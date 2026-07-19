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
