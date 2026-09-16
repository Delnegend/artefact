#![cfg(feature = "gpu")]
//! GPU smoke test: initialise wgpu and run a trivial compute kernel.
//!
//! Skips (passes) when no adapter or required fixture is available, unless
//! `ARTEFACT_REQUIRE_GPU=1`, which turns those conditions into failures. On CI
//! this runs under lavapipe (`mesa-vulkan-drivers`).

use std::path::PathBuf;

use artefact_core::pipeline::gpu::{GpuContext, bytemuck, wgpu};
#[cfg(feature = "simd")]
use artefact_core::{Artefact, JpegSource, ValueCollection};
use wgpu::util::DeviceExt;

fn gpu_is_required() -> bool {
    std::env::var("ARTEFACT_REQUIRE_GPU").is_ok_and(|value| {
        let value = value.to_ascii_lowercase();
        value == "1" || value == "true" || value == "yes"
    })
}

#[cfg(feature = "simd")]
fn gpu_context_or_skip(test: &str) -> Option<GpuContext> {
    match pollster::block_on(GpuContext::new()) {
        Ok(ctx) => {
            eprintln!("adapter: {:?}", ctx.adapter_info());
            Some(ctx)
        }
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "{test}: ARTEFACT_REQUIRE_GPU=1 but no GPU adapter: {e}"
            );
            eprintln!("skipping {test}: {e}");
            None
        }
    }
}

#[cfg(feature = "simd")]
fn fixture_or_skip(test: &str, relative: &str) -> Option<std::path::PathBuf> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    if path.exists() {
        Some(path)
    } else {
        assert!(
            !gpu_is_required(),
            "{test}: ARTEFACT_REQUIRE_GPU=1 but required fixture {relative} is missing"
        );
        eprintln!("skipping {test}: no fixture");
        None
    }
}

#[test]
fn gpu_smoke() {
    pollster::block_on(run());
}

async fn run() {
    let ctx = match GpuContext::new().await {
        Ok(ctx) => ctx,
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "gpu_smoke: ARTEFACT_REQUIRE_GPU=1 but no GPU adapter: {e}"
            );
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

/// End-to-end: the GPU pipeline (`Artefact::process_gpu_with`) must produce an
/// image close to the CPU pipeline on a committed fixture. Requires the `simd`
/// feature so the CPU reference is the production pipeline (the scalar one has
/// a known init discrepancy).
#[cfg(feature = "simd")]
#[test]
fn gpu_process_matches_cpu() {
    const TEST: &str = "gpu_process_matches_cpu";
    let Some(path) = fixture_or_skip(TEST, "tests/fixtures/baseline_420.jpg") else {
        return;
    };
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };

    let make = || {
        Artefact::default()
            .source(JpegSource::File(path.display().to_string()))
            .iterations(ValueCollection::ForAll(3))
    };
    let cpu = make().process().expect("cpu pipeline");
    let gpu = match pollster::block_on(make().process_gpu_with(&ctx)) {
        Ok(image) => image,
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "{TEST}: ARTEFACT_REQUIRE_GPU=1 but GPU solve failed: {e}"
            );
            eprintln!("skipping {TEST}: {e}");
            return;
        }
    };

    let mut max = 0i32;
    let mut sum = 0u64;
    let mut count = 0u64;
    for (a, b) in cpu.pixels().zip(gpu.pixels()) {
        for k in 0..3 {
            let d = (i32::from(a[k]) - i32::from(b[k])).abs();
            max = max.max(d);
            sum += d as u64;
            count += 1;
        }
    }
    let mean = sum as f64 / count as f64;
    eprintln!("gpu_process_matches_cpu: max={max} mean={mean:.3}");
    assert!(max <= 8, "max channel diff {max}");
    assert!(mean < 1.0, "mean channel diff {mean}");
}

/// Exercise a committed 17x9 4:2:0 JPEG whose luma grid is odd.
/// This covers the subsampled resampling/index edge without adding fixtures.
#[cfg(feature = "simd")]
#[test]
fn gpu_matches_cpu_on_odd_width() {
    const TEST: &str = "gpu_matches_cpu_on_odd_width";
    let Some(path) = fixture_or_skip(TEST, "tests/fixtures/odd_420.jpg") else {
        return;
    };
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };

    let make = || {
        Artefact::default()
            .source(JpegSource::File(path.display().to_string()))
            .iterations(ValueCollection::ForAll(3))
    };
    let cpu = make().process().expect("cpu pipeline");
    let gpu = match pollster::block_on(make().process_gpu_with(&ctx)) {
        Ok(image) => image,
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "{TEST}: ARTEFACT_REQUIRE_GPU=1 but GPU solve failed: {e}"
            );
            eprintln!("skipping {TEST}: {e}");
            return;
        }
    };

    assert_eq!(cpu.dimensions(), (17, 9));
    assert_eq!(gpu.dimensions(), cpu.dimensions());

    let mut max = 0i32;
    let mut sum = 0u64;
    let mut count = 0u64;
    for (a, b) in cpu.pixels().zip(gpu.pixels()) {
        for k in 0..3 {
            let d = (i32::from(a[k]) - i32::from(b[k])).abs();
            max = max.max(d);
            sum += d as u64;
            count += 1;
        }
    }
    let mean = sum as f64 / count as f64;
    eprintln!("gpu_matches_cpu_on_odd_width: max={max} mean={mean:.3}");
    assert!(max <= 8, "max channel diff {max}");
    assert!(mean < 1.0, "mean channel diff {mean}");
}
