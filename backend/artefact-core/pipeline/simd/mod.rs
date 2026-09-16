mod adaptive_width;
mod coef;
mod dct_gradient;
mod fista;
mod projection;
mod run;
mod step;
mod tgv;
pub mod traits;
mod tv;

use rayon::prelude::*;

use crate::{
    jpeg::Coefficient, pipeline::simd::dct_gradient::dct_gradient, utils::aligned::AlignedF32,
};
use coef::SIMDCoef;

// Public entry points for the reference TV kernels + widths (used by benches/tests).
pub use adaptive_width::{AdaptiveWidth, adaptive_runs, uniform_runs};
pub use tgv::tgv_gradient;
pub use tv::tv_gradient;

/// Solve for the smoothest image consistent with the JPEG's quantized DCT
/// coefficients, using the SIMD pipeline with adaptive lane widths.
#[allow(unused)]
pub fn solve(
    nchannel: usize,
    coefs: Vec<Coefficient>,
    weight: f32,
    pweight: [f32; 3],
    iterations: usize,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
) -> Vec<AlignedF32> {
    let coefs = coefs
        .into_par_iter()
        .map(SIMDCoef::from)
        .collect::<Vec<_>>();
    let mut auxs = fista::init_channel_states(
        max_rounded_px_w,
        max_rounded_px_h,
        max_rounded_px_count,
        &coefs,
    );
    let radius = fista::box_radius(max_rounded_px_count);
    // `adaptive_runs` emits 64/32/16-lane `std::simd` vectors, which are
    // miscompiled under wasm-simd128 (they produce NaN that poisons the whole
    // channel); wasm uses the uniform 8-lane tiling instead.
    #[cfg(target_arch = "wasm32")]
    let widths = uniform_runs(max_rounded_px_w);
    #[cfg(not(target_arch = "wasm32"))]
    let widths = adaptive_runs(max_rounded_px_w);
    // Shared per-pixel norm scratch for the TV/TGV kernels (reused for both).
    let mut norm = AlignedF32::zeros(max_rounded_px_count);
    fista::run_fista(
        &mut auxs,
        &coefs,
        iterations,
        max_rounded_px_count,
        radius,
        |coefs, auxs, step_size| {
            step::solver_step(
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                nchannel,
                coefs,
                auxs,
                &mut norm,
                step_size,
                weight,
                &pweight,
                dct_gradient,
                |w, h, nch, auxs, norm| tv_gradient(w, h, nch, auxs, &widths, norm),
                |w, h, nch, auxs, alpha, norm| tgv_gradient(w, h, nch, auxs, alpha, &widths, norm),
                projection::project_onto_box,
            );
        },
    );

    auxs.into_par_iter().map(|aux| aux.fdata).collect()
}
