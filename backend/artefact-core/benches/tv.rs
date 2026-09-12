// TV implementations benchmark: shared width-generic f32x8 (compute_step_tv)
// vs f32x64 8x8 block (compute_step_tv_simd_64) vs rayon-parallel f32x8
// (compute_step_tv_simd_par), plus the adaptive mixed-width paths. Run with:
//   cargo bench --features bench --bench bench -- tv

use std::hint::black_box;

use artefact_core::pipeline::simd8::{
    AdaptiveWidth, compute_step_tv, compute_step_tv2, uniform_widths,
};
use artefact_core::{Aux, PixelDifference};
use criterion::Criterion;

use super::{tv_par, tv_simd64};

/// 1600x1200 — same as assets/sample.png; 3 channels (YCbCr) like a joint solve.
const W: u32 = 1600;
const H: u32 = 1200;
/// A width whose tail needs 32/16/8-wide runs, exercising every generic width.
const WM: u32 = 1592;
const NCH: usize = 3;

fn make_auxs(w: u32, h: u32) -> Vec<Aux> {
    let count = (w * h) as usize;
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

/// Mixed 64/32/16/8 tiling of a row (like `get_adaptive_widths`, but guaranteed
/// to include every width even for MCU-aligned fixture widths).
fn mixed_widths(w: u32) -> Vec<AdaptiveWidth> {
    let mut out = Vec::new();
    let mut x = 0;
    while x + 64 <= w {
        out.push(AdaptiveWidth::X64(x));
        x += 64;
    }
    if x + 32 <= w {
        out.push(AdaptiveWidth::X32(x));
        x += 32;
    }
    if x + 16 <= w {
        out.push(AdaptiveWidth::X16(x));
        x += 16;
    }
    while x + 8 <= w {
        out.push(AdaptiveWidth::X8(x));
        x += 8;
    }
    out
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
    let mut auxs = make_auxs(W, H);
    let mut auxs_mixed = make_auxs(WM, H);
    let widths = uniform_widths(W);
    let mixed = mixed_widths(WM);

    let mut group = c.benchmark_group("tv");

    group.bench_function("f32x8 (compute_step_tv)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs));
            compute_step_tv(W, H, NCH, black_box(&mut auxs), black_box(&widths));
        })
    });

    group.bench_function("mixed widths (compute_step_tv)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs_mixed));
            compute_step_tv(WM, H, NCH, black_box(&mut auxs_mixed), black_box(&mixed));
        })
    });

    group.bench_function("f32x8 (compute_step_tv2)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs));
            compute_step_tv2(W, H, NCH, black_box(&mut auxs), 0.3, black_box(&widths));
        })
    });

    group.bench_function("mixed widths (compute_step_tv2)", |b| {
        b.iter(|| {
            reset(black_box(&mut auxs_mixed));
            compute_step_tv2(
                WM,
                H,
                NCH,
                black_box(&mut auxs_mixed),
                0.3,
                black_box(&mixed),
            );
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
