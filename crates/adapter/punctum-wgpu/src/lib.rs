//! Winit input and wgpu runtime integration.

#![forbid(unsafe_code)]

mod input;
mod runtime;

pub use input::{
    WinitCommittedTextSnapshot, WinitKeyEventSnapshot, normalize_committed_text,
    normalize_key_event,
};
pub use runtime::{GpuRuntime, GpuRuntimeError, PresentOutcome};
