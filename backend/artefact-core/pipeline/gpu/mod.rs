//! GPU (`wgpu`) backend.
//!
//! Everything here is behind the `gpu` feature. The CPU pipelines remain the
//! reference implementation and the fallback when no adapter is available or
//! an image exceeds the device's limits.

pub use bytemuck;
pub use wgpu;

mod layout;
pub mod solver;

pub use solver::solve;

#[cfg(test)]
mod tests;

use std::{fmt, future::Future, pin::Pin};

/// Errors from the GPU backend.
#[derive(Debug)]
pub enum GpuError {
    /// No suitable adapter (no GPU / driver, or the browser lacks WebGPU).
    Unavailable(String),
    /// Adapter or device creation failed.
    Init(String),
    /// The image needs more memory or work than the device's limits allow.
    Unsupported(String),
    /// A compute/readback operation failed.
    Runtime(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(m) => write!(f, "no GPU adapter: {m}"),
            Self::Init(m) => write!(f, "GPU init failed: {m}"),
            Self::Unsupported(m) => write!(f, "image exceeds GPU limits: {m}"),
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
    /// What the adapter supports, which can exceed what `device` was granted.
    #[cfg(test)]
    adapter_limits: wgpu::Limits,
}

impl GpuContext {
    /// Request a high-performance adapter and a device with the adapter's own
    /// buffer and dispatch limits, so large images fit where the hardware allows.
    ///
    /// The future is boxed (and not `Send`) to avoid an auto-trait recursion in
    /// wgpu's `Surface` types when computing `Send` for the opaque async fn.
    ///
    /// # Errors
    /// Returns [`GpuError::Unavailable`] when no adapter is available and
    /// [`GpuError::Init`] when device creation fails.
    pub fn new() -> Pin<Box<dyn Future<Output = Result<Self, GpuError>>>> {
        Self::with_limits(solver_limits)
    }

    /// [`Self::new`] with the device limits derived from the adapter's by `limits`.
    pub(crate) fn with_limits(
        limits: impl Fn(&wgpu::Limits) -> wgpu::Limits + 'static,
    ) -> Pin<Box<dyn Future<Output = Result<Self, GpuError>>>> {
        Box::pin(async move {
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
            tracing::info!(
                name = %info.name,
                backend = ?info.backend,
                device_type = ?info.device_type,
                driver = %info.driver,
                "GPU adapter selected"
            );

            let adapter_limits = adapter.limits();
            let required_limits = limits(&adapter_limits);
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("artefact-gpu"),
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    ..Default::default()
                })
                .await
                .map_err(|e| GpuError::Init(e.to_string()))?;

            Ok(Self {
                device,
                queue,
                info,
                #[cfg(test)]
                adapter_limits,
            })
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

/// Default limits, raised to what the adapter supports for the limits the
/// solver's layout depends on. `wgpu` grants exactly the requested limits, and
/// its defaults cap a storage binding at 128 MiB, while one 4K channel needs
/// ~285 MiB.
pub(crate) fn solver_limits(adapter: &wgpu::Limits) -> wgpu::Limits {
    wgpu::Limits {
        max_buffer_size: adapter.max_buffer_size,
        max_storage_buffer_binding_size: adapter.max_storage_buffer_binding_size,
        max_compute_workgroups_per_dimension: adapter.max_compute_workgroups_per_dimension,
        ..wgpu::Limits::default()
    }
}
