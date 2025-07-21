mod coef;
mod compute_projection;
mod compute_step;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

#[cfg(all(feature = "simd", feature = "simd_std"))]
pub use std::simd::f32x8;
#[cfg(all(feature = "simd", not(feature = "simd_std")))]
pub use wide::f32x8;

use rayon::prelude::*;

use crate::{
    jpeg::Coefficient,
    pipeline_simd_8::{coef::SIMD8Coef, compute_step::compute_step},
    utils::{aux::Aux, macros::mul_add},
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
) -> Vec<Vec<f32>> {
    let coefs: Vec<SIMD8Coef> = coefs.into_par_iter().map(SIMD8Coef::from).collect();

    // Initialize working buffers for each channel
    let mut auxs = (0..nchannel)
        .map(|c| {
            Aux::init(
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                &coefs[c],
            )
        })
        .collect::<Vec<_>>();

    // Radius of [-0.5, 0.5]^(h*w)
    let radius = (max_rounded_px_count as f32).sqrt() / 2.0;
    let mut term = 1.0_f32;

    // Main iteration loop
    for _ in 0..iterations {
        // FISTA update
        let next_term = (1.0 + mul_add!(4.0_f32, term.powi(2), 1.0).sqrt()) / 2.0;
        let factor = (term - 1.0) / next_term;

        auxs.par_iter_mut().for_each(|aux| {
            for i in 0..max_rounded_px_count {
                aux.fista[i] = mul_add!(factor, aux.fdata[i] - aux.fista[i], aux.fdata[i]);
            }
            std::mem::swap(&mut aux.fdata, &mut aux.fista);
        });

        term = next_term;

        // Take a step
        compute_step(
            max_rounded_px_w,
            max_rounded_px_h,
            max_rounded_px_count,
            nchannel,
            &coefs,
            &mut auxs,
            radius / (1.0 + iterations as f32).sqrt(),
            weight,
            &pweight,
        );
    }

    auxs.into_iter().map(|aux| aux.fdata).collect()
}
