//! GPU-vs-CPU equivalence check on the real fixtures.
//!
//! Runs the `wgpu` solver and the SIMD reference solver with identical
//! parameters and compares the resulting pixel-domain iterate. Skips silently
//! when no adapter is available (e.g. CI without lavapipe) or when the fixtures
//! are missing (`just sample`).

use crate::{
    jpeg::{Jpeg, JpegSource},
    utils::aligned::AlignedF32,
};

const WEIGHT: f32 = 0.3;
const PWEIGHT: [f32; 3] = [0.001; 3];

fn fixture_path(suffix: &str) -> String {
    format!(
        "{}/../../assets/sample.{suffix}.input.jpg",
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
    let mut checked = 0;
    for suffix in ["420", "444"] {
        let Some((jpeg, nch, w, h, count)) = decode(suffix) else {
            continue;
        };
        checked += 1;
        let ctx = match pollster::block_on(crate::pipeline::gpu::GpuContext::new()) {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("skipping gpu_matches_simd: {e}");
                return;
            }
        };
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
    if checked == 0 {
        eprintln!("skipping gpu_matches_simd: no fixtures");
    }
}
