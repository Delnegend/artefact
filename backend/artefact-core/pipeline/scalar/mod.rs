mod coef;
mod dct_gradient;
mod projection;
mod step;
mod tgv;
mod tv;

#[cfg(test)]
pub(crate) use tgv::tgv_gradient;

use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::pipeline::scalar::coef::ScalarCoef;
use crate::{
    jpeg::Coefficient,
    utils::{aligned::AlignedF32, auxiliary::Aux, macros::mul_add},
};

/// Solve for the smoothest image consistent with the JPEG's quantized DCT
/// coefficients (scalar reference).
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
    let coefs: Vec<ScalarCoef> = coefs
        .into_par_iter()
        .map(std::convert::Into::into)
        .collect();

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
        step::solver_step(
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

    auxs.into_par_iter().map(|aux| aux.fdata).collect()
}
