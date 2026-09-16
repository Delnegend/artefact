//! GPU-vs-CPU equivalence check on committed fixtures.
//!
//! Runs the `wgpu` solver and the SIMD reference solver with identical
//! parameters and compares the resulting pixel-domain iterate. It skips when
//! no adapter or required fixture is available, unless
//! `ARTEFACT_REQUIRE_GPU=1`, which turns those conditions into failures.
//!
//! Large generated samples remain benchmark-only; this keeps strict `just check`
//! runs independent of optional ignored assets.

use crate::{
    jpeg::{Jpeg, JpegSource},
    utils::aligned::AlignedF32,
};

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
    let nchannel = jpeg.nchannel as usize;
    let (mut w, mut h) = (0, 0);
    for coef in &jpeg.coefs {
        w = w.max(coef.rounded_px_w);
        h = h.max(coef.rounded_px_h);
    }
    Some((jpeg, nchannel, w, h, (w * h) as usize))
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
    let ctx = match pollster::block_on(crate::pipeline::gpu::GpuContext::new()) {
        Ok(ctx) => {
            eprintln!("adapter: {:?}", ctx.adapter_info());
            ctx
        }
        Err(e) => {
            assert!(
                !gpu_is_required(),
                "gpu_matches_simd: ARTEFACT_REQUIRE_GPU=1 but no GPU adapter: {e}"
            );
            eprintln!("skipping gpu_matches_simd: {e}");
            return;
        }
    };
    for (suffix, (jpeg, nch, w, h, count)) in inputs {
        for iterations in [1_usize, 5, 50] {
            let cpu = crate::pipeline::simd::solve(
                nch,
                jpeg.coefs.clone(),
                WEIGHT,
                PWEIGHT,
                iterations,
                w,
                h,
                count,
            );
            let gpu = pollster::block_on(crate::pipeline::gpu::solve(
                &ctx,
                &jpeg.coefs,
                WEIGHT,
                &PWEIGHT,
                iterations,
                w,
                h,
                count,
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
