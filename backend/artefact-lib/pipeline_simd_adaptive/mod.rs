mod adaptive_width;
mod coef;
mod compute_projection;
mod compute_step;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

use crate::{
    jpeg::Coefficient,
    utils::{aux::Aux, macros::mul_add},
};
use adaptive_width::get_adaptive_widths;
use coef::SIMDAdaptiveCoef;
use compute_step::compute_step;
use rayon::prelude::*;

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
    let mut coefs = coefs
        .into_par_iter()
        .map(SIMDAdaptiveCoef::from)
        .collect::<Vec<_>>();

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

    let adaptive_widths = get_adaptive_widths(max_rounded_px_w);

    // Main iteration loop
    for _ in 0..iterations {
        // FISTA update
        let next_term = f32::midpoint(1.0, mul_add!(4.0_f32, term.powi(2), 1.0).sqrt());
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
            &adaptive_widths,
        );
    }

    auxs.into_iter().map(|aux| aux.fdata).collect()
}
