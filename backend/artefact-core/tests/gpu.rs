#![cfg(feature = "gpu")]
//! GPU smoke test: initialise wgpu and run a trivial compute kernel.
//!
//! Skips (passes) when no adapter is available. On CI this runs under
//! lavapipe (`mesa-vulkan-drivers`).

use artefact_core::pipeline::gpu::{GpuContext, bytemuck, wgpu};
use wgpu::util::DeviceExt;

#[test]
fn gpu_smoke() {
    pollster::block_on(run());
}

async fn run() {
    let ctx = match GpuContext::new().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("skipping gpu_smoke: {e}");
            return;
        }
    };
    eprintln!("adapter: {:?}", ctx.adapter_info());

    let device = ctx.device();
    let queue = ctx.queue();

    let input = [1.0_f32, 2.0, 3.0, 4.0];
    let bytes = bytemuck::cast_slice(&input);
    let size = bytes.len() as u64;

    let storage = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("smoke-input"),
        contents: bytes,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("smoke-readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    const SHADER: &str = r"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    data[gid.x] = data[gid.x] * 2.0;
}
";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("smoke-shader"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("smoke-pipeline"),
        layout: None,
        module: &module,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("smoke-bind"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage.as_entire_binding(),
        }],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("smoke-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("smoke-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&storage, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    rx.recv().expect("map channel").expect("buffer map");

    let mapped = slice.get_mapped_range().expect("mapped range");
    let output: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
    assert_eq!(output, [2.0, 4.0, 6.0, 8.0], "doubling kernel output");
}
