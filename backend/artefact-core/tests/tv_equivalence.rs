// Verifies that the TV implementations and width tilings produce the same output:
// the width-generic TV kernel (pipeline/simd bench export) with uniform x8 tiling vs
// the adaptive 64/32/16/8 tiling, plus the bench-local 64/par variants pulled in
// via #[path].
// Requires the `bench` feature. Run with: cargo test --features bench --test tv_equivalence

#![feature(portable_simd)]

#[path = "../benches/tv_par.rs"]
mod tv_par;
#[path = "../benches/tv_simd64.rs"]
mod tv_simd64;

use artefact_core::pipeline::simd::{
    compute_step_tv, compute_step_tv2, get_adaptive_widths, uniform_widths,
};
use artefact_core::{AlignedF32, Aux, PixelDifference};

const W: u32 = 1600;
const H: u32 = 1200;
const NCH: usize = 3;
/// fp accumulation order differs across tiling, so allow tiny drift (last-ULP-ish)
const TOL: f32 = 1e-3;

fn make_auxs() -> Vec<Aux> {
    let count = (W * H) as usize;
    (0..NCH)
        .map(|_| Aux {
            cos: AlignedF32::zeros(count),
            obj_gradient: AlignedF32::zeros(count),
            pixel_diff: PixelDifference {
                x: AlignedF32::zeros(count),
                y: AlignedF32::zeros(count),
            },
            fdata: (0..count)
                .map(|i| ((i * 31) % 251) as f32 / 250.0 - 0.5)
                .collect(),
            fista: AlignedF32::zeros(count),
        })
        .collect()
}

fn clone_auxs(src: &[Aux]) -> Vec<Aux> {
    src.iter()
        .map(|a| Aux {
            cos: a.cos.clone(),
            obj_gradient: a.obj_gradient.clone(),
            pixel_diff: PixelDifference {
                x: a.pixel_diff.x.clone(),
                y: a.pixel_diff.y.clone(),
            },
            fdata: a.fdata.clone(),
            fista: a.fista.clone(),
        })
        .collect()
}

fn max_diff(a: &[Aux], b: &[Aux]) -> f32 {
    let mut m = 0.0_f32;
    for (ca, cb) in a.iter().zip(b) {
        for (x, y) in ca.obj_gradient.iter().zip(cb.obj_gradient.iter()) {
            m = m.max((x - y).abs());
        }
        for (x, y) in ca.pixel_diff.x.iter().zip(cb.pixel_diff.x.iter()) {
            m = m.max((x - y).abs());
        }
        for (x, y) in ca.pixel_diff.y.iter().zip(cb.pixel_diff.y.iter()) {
            m = m.max((x - y).abs());
        }
    }
    m
}

#[test]
fn tv_implementations_match() {
    let base = make_auxs();

    let widths = uniform_widths(W);
    let mut a = clone_auxs(&base);
    compute_step_tv(W, H, NCH, &mut a, &widths);

    let mut b = clone_auxs(&base);
    tv_par::compute_step_tv_simd_par(W, H, NCH, &mut b);

    let mut c = clone_auxs(&base);
    tv_simd64::compute_step_tv_simd_64(W, H, NCH, &mut c);

    let par = max_diff(&a, &b);
    let simd64 = max_diff(&a, &c);

    println!("f32x8 vs rayon-par: max diff = {par:.3e}");
    println!("f32x8 vs f32x64:   max diff = {simd64:.3e}");

    assert!(
        par <= TOL,
        "rayon-par diverged from f32x8 by {par} (tol {TOL})"
    );
    assert!(
        simd64 <= TOL,
        "f32x64 diverged from f32x8 by {simd64} (tol {TOL})"
    );
}

/// The adaptive 64/32/16/8 tiling must match the uniform x8 tiling of the same
/// width-generic kernel (the per-width `tv_inner::<N>` dispatch).
#[test]
fn mixed_widths_match_uniform() {
    let base = make_auxs();
    let uniform = uniform_widths(W);
    let mixed = get_adaptive_widths(W);

    let mut uni_tv = clone_auxs(&base);
    compute_step_tv(W, H, NCH, &mut uni_tv, &uniform);
    let mut mix_tv = clone_auxs(&base);
    compute_step_tv(W, H, NCH, &mut mix_tv, &mixed);
    let tv_diff = max_diff(&uni_tv, &mix_tv);

    let mut uni_tv2 = clone_auxs(&base);
    compute_step_tv2(W, H, NCH, &mut uni_tv2, 0.3, &uniform);
    let mut mix_tv2 = clone_auxs(&base);
    compute_step_tv2(W, H, NCH, &mut mix_tv2, 0.3, &mixed);
    let tv2_diff = max_diff(&uni_tv2, &mix_tv2);

    println!("tv  uniform vs mixed: max diff = {tv_diff:.3e}");
    println!("tv2 uniform vs mixed: max diff = {tv2_diff:.3e}");

    assert!(
        tv_diff <= TOL,
        "tv mixed tiling diverged by {tv_diff} (tol {TOL})"
    );
    assert!(
        tv2_diff <= TOL,
        "tv2 mixed tiling diverged by {tv2_diff} (tol {TOL})"
    );
}
