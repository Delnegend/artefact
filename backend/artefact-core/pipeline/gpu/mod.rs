//! GPU (`wgpu`) backend.
//!
//! Everything here is behind the `gpu` feature. The CPU pipelines remain the
//! reference implementation and the fallback when no adapter is available.

pub use bytemuck;
pub use wgpu;

pub mod solver;

pub use solver::solve;

#[cfg(test)]
mod tests;

use std::{fmt, future::Future, pin::Pin};

/// Errors from GPU initialisation.
#[derive(Debug)]
pub enum GpuError {
    /// No suitable adapter (no GPU / driver, or the browser lacks WebGPU).
    Unavailable(String),
    /// Adapter or device creation failed.
    Init(String),
    /// A compute/readback operation failed.
    Runtime(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(m) => write!(f, "no GPU adapter: {m}"),
            Self::Init(m) => write!(f, "GPU init failed: {m}"),
            Self::Runtime(m) => write!(f, "GPU compute failed: {m}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// An initialised GPU device + queue, reused across solves.
pub struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
}

impl GpuContext {
    /// Request a high-performance adapter and a device with default limits.
    ///
    /// The future is boxed (and not `Send`) to avoid an auto-trait recursion in
    /// wgpu's `Surface` types when computing `Send` for the opaque async fn.
    ///
    /// # Errors
    /// Returns [`GpuError::Unavailable`] when no adapter is available and
    /// [`GpuError::Init`] when device creation fails.
    pub fn new() -> Pin<Box<dyn Future<Output = Result<Self, GpuError>>>> {
        Box::pin(Self::create())
    }

    async fn create() -> Result<Self, GpuError> {
        #[cfg(target_arch = "wasm32")]
        let backends = wgpu::Backends::BROWSER_WEBGPU;
        #[cfg(not(target_arch = "wasm32"))]
        let backends = wgpu::Backends::all();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Unavailable(e.to_string()))?;

        let info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("artefact-gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| GpuError::Init(e.to_string()))?;

        Ok(Self {
            device,
            queue,
            info,
        })
    }

    #[must_use]
    pub const fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[must_use]
    pub const fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[must_use]
    pub const fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.info
    }
}
