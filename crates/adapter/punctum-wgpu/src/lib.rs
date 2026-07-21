//! Winit input and wgpu runtime integration.

#![forbid(unsafe_code)]

mod input;
mod runtime;

use winit::event_loop::ActiveEventLoop;

pub use input::{
    WinitCommittedTextSnapshot, WinitKeyEventSnapshot, normalize_committed_text,
    normalize_key_event,
};
pub use runtime::{GpuRuntime, GpuRuntimeError, PresentOutcome};

pub fn instance_for_event_loop(event_loop: &ActiveEventLoop) -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_with_display_handle(Box::new(
        event_loop.owned_display_handle(),
    ));
    if std::env::var_os("WSL2_GUI_APPS_ENABLED").is_some() {
        descriptor.backends = wgpu::Backends::VULKAN;
    }
    wgpu::Instance::new(descriptor)
}
