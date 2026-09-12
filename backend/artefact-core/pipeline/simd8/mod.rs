mod coef;
mod compute_step_prob;

use rayon::prelude::*;

use crate::{
    jpeg::Coefficient,
    pipeline::simd8::compute_step_prob::compute_step_prob,
    utils::{aligned::AlignedF32, fista, projection::projection, step},
};
use coef::SIMD8Coef;

// Public entry points for the reference TV kernels + widths (used by benches/tests).
pub use crate::utils::{
    adaptive_width::{AdaptiveWidth, uniform_widths},
    tv::compute_step_tv,
    tv2::compute_step_tv2,
};

#[allow(unused)]
pub fn compute(
    nchannel: usize,
    coefs: Vec<Coefficient>,
    weight: f32,
    pweight: [f32; 3],
    iterations: usize,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
) -> Vec<AlignedF32> {
    let coefs: Vec<SIMD8Coef> = coefs.into_par_iter().map(SIMD8Coef::from).collect();
    let radius = fista::radius(max_rounded_px_count);
    let widths = uniform_widths(max_rounded_px_w);
    let mut auxs = fista::init_auxs(
        max_rounded_px_w,
        max_rounded_px_h,
        max_rounded_px_count,
        &coefs,
    );
    fista::fista_loop(
        &mut auxs,
        &coefs,
        iterations,
        max_rounded_px_count,
        radius,
        |coefs, auxs, step_size| {
            step::step(
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                nchannel,
                coefs,
                auxs,
                step_size,
                weight,
                &pweight,
                compute_step_prob,
                |w, h, nch, auxs| compute_step_tv(w, h, nch, auxs, &widths),
                |w, h, nch, auxs, alpha| compute_step_tv2(w, h, nch, auxs, alpha, &widths),
                projection,
            );
        },
    );

    auxs.into_par_iter().map(|aux| aux.fdata).collect()
}
