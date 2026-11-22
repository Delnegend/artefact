//! GPU-vs-CPU equivalence checks.
//!
//! Runs the `wgpu` solver and the SIMD reference solver with identical
//! parameters and compares the resulting pixel-domain iterate. Tests skip when
//! no adapter or required fixture is available, unless `ARTEFACT_REQUIRE_GPU=1`,
//! which turns those conditions into failures.
//!
//! The 4K cases build a synthetic image in memory (issue #55), so strict
//! `just check` runs stay independent of optional ignored assets.

use crate::{
    jpeg::{Coefficient, Jpeg, JpegSource},
    pipeline::gpu::{GpuContext, GpuError, solver_limits},
    utils::aligned::AlignedF32,
};
use zune_jpeg::sample_factor::SampleFactor;

const WEIGHT: f32 = 0.3;
const PWEIGHT: [f32; 3] = [0.001; 3];

fn gpu_is_required() -> bool {
    std::env::var("ARTEFACT_REQUIRE_GPU").is_ok_and(|value| {
        let value = value.to_ascii_lowercase();
        value == "1" || value == "true" || value == "yes"
    })
}

fn fixture_path(suffix: &str) -> String {
    format!(
        "{}/tests/fixtures/baseline_{suffix}.jpg",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn decode(suffix: &str) -> Option<(Jpeg, usize, u32, u32, usize)> {
    let path = fixture_path(suffix);
    if !std::path::Path::new(&path).exists() {
        return None;
    }
    let jpeg = Jpeg::from(JpegSource::File(path)).ok()?;
    let nchannel = usize::try_from(jpeg.nchannel).ok()?;
    let (mut w, mut h) = (0u32, 0u32);
    for coef in &jpeg.coefs {
        w = w.max(coef.rounded_px_w);
        h = h.max(coef.rounded_px_h);
    }
    let count = w.checked_mul(h).and_then(|px| usize::try_from(px).ok())?;
    Some((jpeg, nchannel, w, h, count))
}

fn max_diff(a: &[AlignedF32], b: &[AlignedF32]) -> f32 {
    let mut m = 0.0_f32;
    for (ca, cb) in a.iter().zip(b) {
        for (x, y) in ca.iter().zip(cb.iter()) {
            m = m.max((x - y).abs());
        }
    }
    m
}

fn context_or_skip(test: &str, ctx: Result<GpuContext, GpuError>) -> Option<GpuContext> {
    match ctx {
        Ok(ctx) => {
            eprintln!("{test}: adapter {:?}", ctx.adapter_info());
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

/// A JPEG-like component with `factor`x`factor` subsampling. Each block has
/// a DC level that varies across the image and one AC term, so neighbouring
/// blocks differ and the solver has real work at every block boundary.
/// Small test sizes cannot overflow, so wrapping arithmetic is exact here.
fn synthetic_component(w: u32, h: u32, factor: SampleFactor) -> Coefficient {
    let (block_w, block_h) = (w.div_ceil(8), h.div_ceil(8));
    let block_count = block_w.wrapping_mul(block_h);
    let blocks_per_row = usize::try_from(block_w).unwrap();
    let len = usize::try_from(block_count).unwrap().wrapping_mul(64);
    let level = |v: usize| f32::from(u8::try_from(v).unwrap());
    let mut dct_coefs = vec![0.0_f32; len];
    for (i, block) in dct_coefs.as_chunks_mut::<64>().0.iter_mut().enumerate() {
        let (by, bx) = (i.checked_div(blocks_per_row), i.checked_rem(blocks_per_row));
        let (bx, by) = (bx.unwrap(), by.unwrap());
        let dc = bx.wrapping_mul(7).wrapping_add(by.wrapping_mul(3)) % 40;
        block[0] = level(dc) - 20.0;
        block[(bx.wrapping_add(by) % 8).wrapping_add(1)] = level((bx ^ by) % 5) - 2.0;
    }
    Coefficient {
        rounded_px_w: block_w.wrapping_mul(8),
        rounded_px_h: block_h.wrapping_mul(8),
        rounded_px_count: block_count.wrapping_mul(64),
        block_w,
        block_h,
        block_count,
        horizontal_samp_factor: factor,
        vertical_samp_factor: factor,
        dct_coefs,
        quant_table: std::array::from_fn(|k| level(k % 8).max(level(k / 8)) + 4.0),
    }
}

/// 3840x2160 4:2:0: the size that panicked on default device limits (#55).
fn uhd_420() -> Vec<Coefficient> {
    let chroma = synthetic_component(1920, 1080, SampleFactor::Two);
    vec![
        synthetic_component(3840, 2160, SampleFactor::One),
        chroma.clone(),
        chroma,
    ]
}

/// Whether `adapter` can hold the 4K 4:2:0 channel and aux buffers. An adapter
/// that cannot (CI's lavapipe caps a binding at 128 MiB) is a legitimate skip;
/// one that can must run 4K, so a regression to default limits fails here
/// rather than silently skipping.
fn adapter_can_run_uhd(adapter: &wgpu::Limits) -> bool {
    let channel_bytes = 9 * 3840 * 2160 * 4;
    adapter.max_storage_buffer_binding_size >= channel_bytes
        && adapter.max_buffer_size >= channel_bytes
}

#[test]
fn gpu_matches_simd_at_4k() {
    const TEST: &str = "gpu_matches_simd_at_4k";
    let Some(ctx) = context_or_skip(TEST, pollster::block_on(GpuContext::new())) else {
        return;
    };
    if !adapter_can_run_uhd(&ctx.adapter_limits) {
        assert!(
            !gpu_requires_uhd(),
            "{TEST}: ARTEFACT_REQUIRE_UHD=1 but the adapter cannot hold a 4K channel"
        );
        eprintln!("skipping {TEST}: adapter limits too small for 4K");
        return;
    }
    let coefs = uhd_420();
    let gpu = pollster::block_on(crate::pipeline::gpu::solve(
        &ctx, &coefs, WEIGHT, &PWEIGHT, 3, 3840, 2160,
    ))
    .unwrap_or_else(|e| panic!("{TEST}: the adapter supports 4K but the solve failed: {e}"));
    let cpu = crate::pipeline::simd::solve(3, coefs, WEIGHT, PWEIGHT, 3, 3840, 2160);
    assert_eq!(gpu.len(), 3);
    assert!(gpu.iter().all(|c| c.len() == 3840 * 2160));
    let diff = max_diff(&cpu, &gpu);
    eprintln!("{TEST}: max_diff={diff}");
    assert!(diff < 5.0, "max diff {diff}");
}

/// Opt-in strictness for machines known to have a capable GPU.
fn gpu_requires_uhd() -> bool {
    std::env::var("ARTEFACT_REQUIRE_UHD").is_ok_and(|v| v == "1")
}

/// Run the GPU and CPU solvers on `coefs` and return the largest difference.
fn gpu_vs_cpu(
    test: &str,
    coefs: &[Coefficient],
    (w, h): (u32, u32),
    (weight, pweight): (f32, [f32; 3]),
    iterations: usize,
) -> Option<f32> {
    let ctx = context_or_skip(test, pollster::block_on(GpuContext::new()))?;
    let gpu = pollster::block_on(crate::pipeline::gpu::solve(
        &ctx, coefs, weight, &pweight, iterations, w, h,
    ))
    .unwrap_or_else(|e| panic!("{test}: {e}"));
    let cpu = crate::pipeline::simd::solve(
        coefs.len(),
        coefs.to_vec(),
        weight,
        pweight,
        iterations,
        w,
        h,
    );
    Some(max_diff(&cpu, &gpu))
}

/// After one iteration only a few terms have acted, so a broken kernel cannot
/// hide inside the loose tolerance used for converged results: the GPU and CPU
/// must agree to float rounding. This pins the first FISTA step, the
/// regularisers, and the DCT projection channel by channel.
#[test]
fn gpu_matches_cpu_tightly_after_one_iteration() {
    let coefs = yuv420(96, 64);
    let Some(diff) = gpu_vs_cpu(
        "gpu_matches_cpu_tightly_after_one_iteration",
        &coefs,
        (96, 64),
        (WEIGHT, PWEIGHT),
        1,
    ) else {
        return;
    };
    assert!(diff < 0.01, "max diff after one iteration: {diff}");
}

/// Per-channel fidelity weights must reach their own channel. Running at one
/// iteration isolates the data-fidelity term before second-step `safe_div`
/// divergence sets in, so GPU and CPU agree to ~1e-5; dropping or swapping a
/// channel's weight shows up cleanly here.
#[test]
fn per_channel_fidelity_weights_apply_to_their_own_channel() {
    let coefs = yuv420(96, 64);
    for (name, weights) in [
        ("fidelity only on Y", [0.5, 0.0, 0.0]),
        ("fidelity only on Cb", [0.0, 0.5, 0.0]),
        ("fidelity only on Cr", [0.0, 0.0, 0.5]),
        ("fidelity on all, unequal", [0.2, 0.05, 0.4]),
    ] {
        let Some(diff) = gpu_vs_cpu(
            "per_channel_fidelity_weights_apply_to_their_own_channel",
            &coefs,
            (96, 64),
            (0.0, weights),
            1,
        ) else {
            return;
        };
        assert!(diff < 0.01, "{name}: max diff {diff}");
    }
}

/// Every channel's final state, not just the first, must be read back from the
/// field holding the latest iterate. Two iterations leave the iterate in field 0
/// and one leaves it in field 1 (odd vs even ping-pong), and checking per
/// channel ensures Cb/Cr are not left on field 0.
#[test]
fn readback_returns_the_latest_iterate_for_odd_and_even_counts() {
    let coefs = yuv420(96, 64);
    for (iterations, limit) in [(1, 0.01), (2, 1.5), (3, 1.5), (4, 1.5)] {
        let Some(diff) = gpu_vs_cpu(
            "readback_returns_the_latest_iterate_for_odd_and_even_counts",
            &coefs,
            (96, 64),
            (WEIGHT, PWEIGHT),
            iterations,
        ) else {
            return;
        };
        assert!(diff < limit, "{iterations} iterations: max diff {diff}");
    }
}

/// `captured_error` decides whether an image is "too big" (fall back quietly) or
/// "broken" (report it); the mapping must not be swapped or swallowed.
#[test]
fn captured_gpu_errors_map_to_the_right_variant() {
    let validation = || wgpu::Error::Validation {
        source: "bad".into(),
        description: "bad".into(),
    };
    let oom = || wgpu::Error::OutOfMemory {
        source: "full".into(),
    };
    let internal = || wgpu::Error::Internal {
        source: "boom".into(),
        description: "boom".into(),
    };
    assert!(crate::pipeline::gpu::solver::captured_error(None, None, None).is_none());
    assert!(matches!(
        crate::pipeline::gpu::solver::captured_error(Some(oom()), None, None),
        Some(GpuError::Unsupported(m)) if m.contains("out of GPU memory")
    ));
    assert!(matches!(
        crate::pipeline::gpu::solver::captured_error(None, Some(validation()), None),
        Some(GpuError::Runtime(_))
    ));
    assert!(matches!(
        crate::pipeline::gpu::solver::captured_error(None, None, Some(internal())),
        Some(GpuError::Runtime(_))
    ));
    // Running out of memory wins: it is recoverable by falling back, and
    // validation errors that follow an allocation failure are its symptoms.
    assert!(matches!(
        crate::pipeline::gpu::solver::captured_error(Some(oom()), Some(validation()), None),
        Some(GpuError::Unsupported(_))
    ));
}

/// A genuine `wgpu` validation error raised inside the captured scope must come
/// back as an error value instead of panicking the process.
#[test]
fn wgpu_validation_errors_are_returned_not_panicked() {
    const TEST: &str = "wgpu_validation_errors_are_returned_not_panicked";
    let Some(ctx) = context_or_skip(TEST, pollster::block_on(GpuContext::new())) else {
        return;
    };
    let device = ctx.device();
    let result: Result<(), GpuError> = pollster::block_on(
        crate::pipeline::gpu::solver::with_captured_errors(device, async {
            // Zero-sized storage buffers are invalid to bind but legal to create;
            // an unsupported usage combination reliably raises a validation error.
            let _ = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("invalid"),
                size: 4,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            });
            Ok(())
        }),
    );
    assert!(
        matches!(result, Err(GpuError::Runtime(_))),
        "expected a captured validation error, got {result:?}"
    );
}

/// Before #55 the solver ran on default limits and panicked inside `wgpu`;
/// now an image too large for the device is a returned error, which lets
/// `process_auto` fall back to the CPU.
#[test]
fn gpu_rejects_image_larger_than_device_limits() {
    const TEST: &str = "gpu_rejects_image_larger_than_device_limits";
    let ctx = pollster::block_on(GpuContext::with_limits(|_| wgpu::Limits::default()));
    let Some(ctx) = context_or_skip(TEST, ctx) else {
        return;
    };
    let coefs = uhd_420();
    let result = pollster::block_on(crate::pipeline::gpu::solve(
        &ctx, &coefs, WEIGHT, &PWEIGHT, 1, 3840, 2160,
    ));
    match result {
        Err(GpuError::Unsupported(message)) => {
            assert!(message.contains("298598400"), "{message}");
        }
        other => panic!("expected an unsupported-image error, got {other:?}"),
    }
}

#[test]
fn gpu_matches_simd() {
    let mut inputs = Vec::new();
    let mut missing = Vec::new();
    for suffix in ["420", "444"] {
        match decode(suffix) {
            Some(input) => inputs.push((suffix, input)),
            None => missing.push(suffix),
        }
    }
    assert!(
        !gpu_is_required() || missing.is_empty(),
        "gpu_matches_simd: ARTEFACT_REQUIRE_GPU=1 but fixture(s) missing: {}",
        missing.join(", ")
    );
    if inputs.is_empty() {
        eprintln!("skipping gpu_matches_simd: no fixtures");
        return;
    }
    let Some(ctx) = context_or_skip("gpu_matches_simd", pollster::block_on(GpuContext::new()))
    else {
        return;
    };
    for (suffix, (jpeg, nch, w, h, _count)) in inputs {
        for iterations in [1_usize, 5, 50] {
            let cpu = crate::pipeline::simd::solve(
                nch,
                jpeg.coefs.clone(),
                WEIGHT,
                PWEIGHT,
                iterations,
                w,
                h,
            );
            let gpu = pollster::block_on(crate::pipeline::gpu::solve(
                &ctx,
                &jpeg.coefs,
                WEIGHT,
                &PWEIGHT,
                iterations,
                w,
                h,
            ))
            .expect("gpu solve");
            let diff = max_diff(&cpu, &gpu);
            eprintln!("{suffix} iterations={iterations} max_diff={diff}");
            assert!(
                diff < 5.0,
                "{suffix} iterations={iterations}: max diff {diff}"
            );
        }
    }
}

/// Run `coefs` on a device capped at `cap` workgroups per dimension and check it
/// matches the CPU. Real images only fold dispatches past 65,535 workgroups
/// (~5.6 MP), so a small cap is the only way to run the fold paths (row-folded
/// `flat_group` indexing and strided L2 reduction) in a fast test.
///
/// A grid holds at most `cap * cap` workgroups, so `cap` must be large enough
/// to fit (`fits`) yet small enough that the busiest kernel needs more than one
/// row (`folds`); both are asserted, so a future size change cannot turn the
/// test into a silent no-op.
fn assert_matches_cpu_with_cap(
    test: &str,
    coefs: &[Coefficient],
    (w, h): (u32, u32),
    cap: u32,
    busiest_kernel_groups: u32,
) {
    assert!(
        busiest_kernel_groups > cap,
        "{test}: cap {cap} does not fold {busiest_kernel_groups} workgroups"
    );
    assert!(
        busiest_kernel_groups <= cap.saturating_mul(cap),
        "{test}: cap {cap} cannot fit {busiest_kernel_groups} workgroups"
    );
    let ctx = pollster::block_on(GpuContext::with_limits(move |adapter| wgpu::Limits {
        max_compute_workgroups_per_dimension: cap.min(adapter.max_compute_workgroups_per_dimension),
        ..solver_limits(adapter)
    }));
    let Some(ctx) = context_or_skip(test, ctx) else {
        return;
    };
    let nch = coefs.len();
    let gpu = pollster::block_on(crate::pipeline::gpu::solve(
        &ctx, coefs, WEIGHT, &PWEIGHT, 5, w, h,
    ))
    .unwrap_or_else(|e| panic!("{test}: cap {cap}: {e}"));
    let cpu = crate::pipeline::simd::solve(nch, coefs.to_vec(), WEIGHT, PWEIGHT, 5, w, h);
    let diff = max_diff(&cpu, &gpu);
    eprintln!("{test}: cap={cap} max_diff={diff}");
    assert!(diff < 5.0, "{test}: cap {cap}: max diff {diff}");
}

fn yuv420(w: u32, h: u32) -> Vec<Coefficient> {
    let chroma = synthetic_component(w.div_ceil(2), h.div_ceil(2), SampleFactor::Two);
    vec![
        synthetic_component(w, h, SampleFactor::One),
        chroma.clone(),
        chroma,
    ]
}

/// Dispatch folding must not change the answer. 96x64 4:2:0 needs 72 pixel
/// groups and 96 luma blocks; caps from 10 up fit, and caps that do not divide
/// the counts leave a partly filled last row.
#[test]
fn folded_dispatch_matches_cpu_when_workgroups_are_scarce() {
    let coefs = yuv420(96, 64);
    for cap in [10, 11, 13, 16, 32] {
        assert_matches_cpu_with_cap(
            "folded_dispatch_matches_cpu_when_workgroups_are_scarce",
            &coefs,
            (96, 64),
            cap,
            96,
        );
    }
}

/// The L2 reduction is capped at `reduce_groups`, which follows the device
/// limit, and strides over the rest of the image. With cap 14, 128x96 has
/// 48 chunks for 14 groups, so each group sums several and the result must
/// still equal the unstrided one.
#[test]
fn strided_reduction_matches_cpu_with_few_groups() {
    assert_matches_cpu_with_cap(
        "strided_reduction_matches_cpu_with_few_groups",
        &yuv420(128, 96),
        (128, 96),
        14,
        192,
    );
}

/// A 1-channel (grayscale) image has a single `z` layer and different aux
/// offsets than 3 channels.
#[test]
fn folded_dispatch_matches_cpu_for_one_channel() {
    let gray = vec![synthetic_component(104, 72, SampleFactor::One)];
    assert_matches_cpu_with_cap(
        "folded_dispatch_matches_cpu_for_one_channel",
        &gray,
        (104, 72),
        11,
        117,
    );
}

/// 4:1:1 chroma (horizontal factor 4, full height) on an image that is not a
/// multiple of the 32-pixel MCU, as the decoder pads it: 72x40 pads to 96x40
/// luma, and 96/4 = 24 is a whole number of 8-pixel blocks.
#[test]
fn folded_dispatch_matches_cpu_for_411_and_ragged_sizes() {
    let chroma = |w, h| {
        let mut c = synthetic_component(w, h, SampleFactor::One);
        c.horizontal_samp_factor = SampleFactor::Four;
        c
    };
    let coefs = vec![
        synthetic_component(96, 40, SampleFactor::One),
        chroma(24, 40),
        chroma(24, 40),
    ];
    // 96x40 = 3,840 px -> 15 plane groups, 45 pixel groups; 60 luma blocks.
    assert_matches_cpu_with_cap(
        "folded_dispatch_matches_cpu_for_411_and_ragged_sizes",
        &coefs,
        (96, 40),
        8,
        60,
    );
}
