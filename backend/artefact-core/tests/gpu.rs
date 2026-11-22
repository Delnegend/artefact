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
    pollster::block_on(async {
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
        let size = u64::try_from(bytes.len()).unwrap_or(0);

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

        let shader = r"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(4)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    data[gid.x] = data[gid.x] * 2.0;
}
";
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("smoke-shader"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("smoke-pipeline"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        assert!(rx.recv().is_ok_and(|r| r.is_ok()), "buffer map");

        let mapped = slice.get_mapped_range().expect("mapped range");
        let output: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
        assert_eq!(output, [2.0, 4.0, 6.0, 8.0], "doubling kernel output");
    });
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
            sum += u64::from(d.unsigned_abs());
            count += 1;
        }
    }
    let mean = f64::from(u32::try_from(sum).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(count).unwrap_or(u32::MAX));
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
            sum += u64::from(d.unsigned_abs());
            count += 1;
        }
    }
    let mean = f64::from(u32::try_from(sum).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    eprintln!("gpu_matches_cpu_on_odd_width: max={max} mean={mean:.3}");
    assert!(max <= 8, "max channel diff {max}");
    assert!(mean < 1.0, "mean channel diff {mean}");
}

/// Max and mean absolute per-sample difference between two equally sized buffers.
#[cfg(feature = "simd")]
fn diff_stats(a: &[u16], b: &[u16]) -> (u32, f64) {
    assert_eq!(a.len(), b.len(), "buffer length mismatch");
    let mut max = 0u32;
    let mut sum = 0u64;
    for (&x, &y) in a.iter().zip(b) {
        let d = u32::from(x.abs_diff(y));
        max = max.max(d);
        sum = sum.saturating_add(u64::from(d));
    }
    let count = u64::try_from(a.len()).unwrap_or(u64::MAX).max(1);
    let mean = f64::from(u32::try_from(sum).unwrap_or(u32::MAX))
        / f64::from(u32::try_from(count).unwrap_or(u32::MAX));
    (max, mean)
}

#[cfg(feature = "simd")]
fn widen(raw: &[u8]) -> Vec<u16> {
    raw.iter().map(|&v| u16::from(v)).collect()
}

/// Unwraps a GPU result, or skips (returns `None`) unless `ARTEFACT_REQUIRE_GPU=1`.
#[cfg(feature = "simd")]
fn gpu_ok_or_skip<T, E: std::fmt::Display>(test: &str, result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "{test}: ARTEFACT_REQUIRE_GPU=1 but GPU solve failed: {e}"
            );
            eprintln!("skipping {test}: {e}");
            None
        }
    }
}

#[cfg(feature = "simd")]
fn file_source(path: &std::path::Path) -> JpegSource {
    JpegSource::File(path.display().to_string())
}

/// Every decodable fixture, including grayscale (1 channel) and 4:1:1
/// (horizontal factor 4), neither of which previously ran on the GPU.
#[cfg(feature = "simd")]
#[test]
fn gpu_matches_cpu_on_every_decodable_fixture() {
    const TEST: &str = "gpu_matches_cpu_on_every_decodable_fixture";
    const FIXTURES: [&str; 11] = [
        "baseline_411.jpg",
        "baseline_420.jpg",
        "baseline_422.jpg",
        "baseline_444.jpg",
        "gray.jpg",
        "odd_411.jpg",
        "odd_420.jpg",
        "progressive_420.jpg",
        "progressive_444.jpg",
        "restart_420.jpg",
        "progressive_restart_420.jpg",
    ];
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };
    for name in FIXTURES {
        let Some(path) = fixture_or_skip(TEST, &format!("tests/fixtures/{name}")) else {
            continue;
        };
        let make = || {
            Artefact::default()
                .source(file_source(&path))
                .iterations(ValueCollection::ForAll(3))
        };
        let cpu = make().process().expect("cpu pipeline");
        let Some(gpu) = gpu_ok_or_skip(TEST, pollster::block_on(make().process_gpu_with(&ctx)))
        else {
            return;
        };
        assert_eq!(gpu.dimensions(), cpu.dimensions(), "{name}: dimensions");
        let (max, mean) = diff_stats(&widen(cpu.as_raw()), &widen(gpu.as_raw()));
        eprintln!("{TEST}: {name} max={max} mean={mean:.3}");
        assert!(max <= 8, "{name}: max channel diff {max}");
        assert!(mean < 1.0, "{name}: mean channel diff {mean}");
    }
}

/// Solves a (possibly subsampled) chroma component alone with nchannel=1 on the
/// full image grid; distinct per-channel iterations make each channel differ.
#[cfg(feature = "simd")]
#[test]
fn gpu_matches_cpu_with_separate_components() {
    const TEST: &str = "gpu_matches_cpu_with_separate_components";
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };
    for name in [
        "baseline_420.jpg",
        "baseline_422.jpg",
        "odd_420.jpg",
        "gray.jpg",
    ] {
        let Some(path) = fixture_or_skip(TEST, &format!("tests/fixtures/{name}")) else {
            continue;
        };
        let make = || {
            Artefact::default()
                .source(file_source(&path))
                .iterations(ValueCollection::ForEach([1, 3, 5]))
                .separate_components(true)
        };
        let cpu = make().process().expect("cpu pipeline");
        let Some(gpu) = gpu_ok_or_skip(TEST, pollster::block_on(make().process_gpu_with(&ctx)))
        else {
            return;
        };
        assert_eq!(gpu.dimensions(), cpu.dimensions(), "{name}: dimensions");
        let (max, mean) = diff_stats(&widen(cpu.as_raw()), &widen(gpu.as_raw()));
        eprintln!("{TEST}: {name} max={max} mean={mean:.3}");
        assert!(max <= 8, "{name}: max channel diff {max}");
        assert!(mean < 1.0, "{name}: mean channel diff {mean}");
    }
}

/// The 16-bit GPU path must match the 16-bit CPU path and keep fractions.
#[cfg(feature = "simd")]
#[test]
fn gpu_sixteen_bit_matches_cpu() {
    const TEST: &str = "gpu_sixteen_bit_matches_cpu";
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };
    for name in ["baseline_420.jpg", "gray.jpg"] {
        let Some(path) = fixture_or_skip(TEST, &format!("tests/fixtures/{name}")) else {
            continue;
        };
        let make = || {
            Artefact::default()
                .source(file_source(&path))
                .iterations(ValueCollection::ForAll(3))
        };
        let cpu = make().process_16().expect("cpu pipeline");
        let Some(gpu) = gpu_ok_or_skip(TEST, pollster::block_on(make().process_gpu_with_16(&ctx)))
        else {
            return;
        };
        assert_eq!(gpu.dimensions(), cpu.dimensions(), "{name}: dimensions");
        let (max, mean) = diff_stats(cpu.as_raw(), gpu.as_raw());
        eprintln!("{TEST}: {name} max={max} mean={mean:.3}");
        assert!(max <= 8 * 257, "{name}: max diff {max}");
        assert!(mean < 257.0, "{name}: mean diff {mean}");
        assert!(
            gpu.as_raw().iter().any(|v| v % 257 != 0),
            "{name}: 16-bit GPU output has no fractional precision"
        );
    }
}

/// Bad input must be an error (never a panic), and the context must stay usable.
#[cfg(feature = "simd")]
#[test]
fn gpu_rejects_corrupt_input_without_panicking() {
    const TEST: &str = "gpu_rejects_corrupt_input_without_panicking";
    let Some(good) = fixture_or_skip(TEST, "tests/fixtures/baseline_420.jpg") else {
        return;
    };
    let Some(arithmetic) = fixture_or_skip(TEST, "tests/fixtures/arithmetic_420.jpg") else {
        return;
    };
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };

    let bytes = std::fs::read(&good).expect("read fixture");
    let truncated = bytes[..bytes.len() / 2].to_vec();
    let result = pollster::block_on(
        Artefact::default()
            .source(JpegSource::Buffer(truncated))
            .iterations(ValueCollection::ForAll(1))
            .process_gpu_with(&ctx),
    );
    assert!(
        result.is_err(),
        "truncated JPEG should fail on the GPU path"
    );

    let result = pollster::block_on(
        Artefact::default()
            .source(file_source(&arithmetic))
            .iterations(ValueCollection::ForAll(1))
            .process_gpu_with(&ctx),
    );
    assert!(result.is_err(), "arithmetic-coded JPEG should be rejected");

    // The same context must still solve a valid image after the failures.
    let result = pollster::block_on(
        Artefact::default()
            .source(file_source(&good))
            .iterations(ValueCollection::ForAll(1))
            .process_gpu_with(&ctx),
    );
    assert!(
        result.is_ok(),
        "context unusable after error: {:?}",
        result.err().map(|e| e.to_string())
    );
}

#[cfg(feature = "simd")]
#[test]
fn gpu_benchmark_mode_returns_benchmark_error() {
    const TEST: &str = "gpu_benchmark_mode_returns_benchmark_error";
    let Some(path) = fixture_or_skip(TEST, "tests/fixtures/baseline_420.jpg") else {
        return;
    };
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };
    let result = pollster::block_on(
        Artefact::default()
            .source(file_source(&path))
            .iterations(ValueCollection::ForAll(1))
            .benchmark(true)
            .process_gpu_with(&ctx),
    );
    // The criterion GPU bench relies on benchmark mode yielding exactly this error.
    assert!(
        matches!(result, Err(artefact_core::ArtefactError::Benchmark)),
        "benchmark mode must return ArtefactError::Benchmark"
    );
}

/// Solving twice on one context must give bit-identical output.
#[cfg(feature = "simd")]
#[test]
fn gpu_context_is_reusable_and_deterministic() {
    const TEST: &str = "gpu_context_is_reusable_and_deterministic";
    let Some(path) = fixture_or_skip(TEST, "tests/fixtures/baseline_420.jpg") else {
        return;
    };
    let Some(ctx) = gpu_context_or_skip(TEST) else {
        return;
    };
    let make = || {
        Artefact::default()
            .source(file_source(&path))
            .iterations(ValueCollection::ForAll(3))
    };
    let Some(first) = gpu_ok_or_skip(TEST, pollster::block_on(make().process_gpu_with(&ctx)))
    else {
        return;
    };
    let second = pollster::block_on(make().process_gpu_with(&ctx)).expect("second solve");
    assert_eq!(first.dimensions(), second.dimensions());
    assert!(
        first.as_raw() == second.as_raw(),
        "repeated GPU solve on one context differs"
    );
}
