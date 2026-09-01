// TV implementations benchmark: f32x8 (compute_step_tv) vs f32x64 8x8 block
// (compute_step_tv_simd_64) vs rayon-parallel f32x8 (compute_step_tv_simd_par).
// The 64/par variants are experimental (slower) and live here in benches/,
// only the f32x8 one is kept in pipeline/simd8.
// Run with: cargo bench --features bench --bench bench -- tv

use std::hint::black_box;

use artefact_core::pipeline::simd8::compute_step_tv;
use artefact_core::{Aux, PixelDifference};
use criterion::Criterion;

use super::{tv_par, tv_simd64};

/// 1600x1200 — same as assets/sample.png; 3 channels (YCbCr) like a joint solve.
const W: u32 = 1600;
const H: u32 = 1200;
const NCH: usize = 3;

fn make_auxs() -> Vec<Aux> {
    let count = (W * H) as usize;
    (0..NCH)
        .map(|_| Aux {
            cos: vec![0.0; count],
            obj_gradient: vec![0.0; count],
            pixel_diff: PixelDifference {
                x: vec![0.0; count],
                y: vec![0.0; count],
            },
            // deterministic pseudo-gradient so fdata isn't uniform (all-zero g_norm
            // would short-circuit the derivative writes and misrepresent the work)
            fdata: (0..count)
                .map(|i| ((i * 31) % 251) as f32 / 250.0 - 0.5)
                .collect(),
            fista: vec![0.0; count],
        })
        .collect()
}

/// tv only writes obj_gradient + pixel_diff; reset those between iterations so
/// each run measures the same amount of work (fdata stays constant).
fn reset(auxs: &mut [Aux]) {
    for a in auxs.iter_mut() {
        a.obj_gradient.fill(0.0);
        a.pixel_diff.x.fill(0.0);
        a.pixel_diff.y.fill(0.0);
    }
}

pub fn tv_benches(c: &mut Criterion) {
    let mut auxs = make_auxs();
    let mut group = c.benchmark_group("tv");

    group.bench_function("f32x8 (compute_step_tv)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs));
            compute_step_tv(W, H, NCH, black_box(&mut auxs));
        })
    });

    group.bench_function("f32x64 8x8 (compute_step_tv_simd_64)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs));
            tv_simd64::compute_step_tv_simd_64(W, H, NCH, black_box(&mut auxs));
        })
    });

    group.bench_function("rayon f32x8 (compute_step_tv_simd_par)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs));
            tv_par::compute_step_tv_simd_par(W, H, NCH, black_box(&mut auxs));
        })
    });

    group.finish();
}
