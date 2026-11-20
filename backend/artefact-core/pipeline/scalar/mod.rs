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
pub fn solve(
    nchannel: usize,
    coefs: Vec<Coefficient>,
    weight: f32,
    pweight: [f32; 3],
    iterations: usize,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
) -> Vec<AlignedF32> {
    let max_rounded_px_count =
        usize::try_from(max_rounded_px_w.saturating_mul(max_rounded_px_h)).unwrap_or(usize::MAX);
    let coefs: Vec<ScalarCoef> = coefs
        .into_par_iter()
        .map(std::convert::Into::into)
        .collect();

    // Initialize working buffers for each channel
    let mut auxs = coefs
        .iter()
        .take(nchannel)
        .map(|coef| {
            Aux::init(
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                coef,
            )
        })
        .collect::<Vec<_>>();

    let usize_to_f32 = |n: usize| -> f32 {
        let clamped = u32::try_from(n).unwrap_or(u32::MAX);
        let hi = f32::from(u16::try_from(clamped >> 16).unwrap_or(u16::MAX));
        let lo = f32::from(u16::try_from(clamped & 0xFFFF).unwrap_or(0));
        hi.mul_add(65536.0, lo)
    };

    // Radius of [-0.5, 0.5]^(h*w)
    let radius = usize_to_f32(max_rounded_px_count).sqrt() / 2.0;
    let mut term = 1.0_f32;

    // Main iteration loop
    for _ in 0..iterations {
        // FISTA update
        let next_term = f32::midpoint(1.0, mul_add!(4.0_f32, term.powi(2), 1.0).sqrt());
        let factor = (term - 1.0) / next_term;

        auxs.par_iter_mut().for_each(|aux| {
            for (fista, &fdata) in aux
                .fista
                .iter_mut()
                .zip(aux.fdata.iter())
                .take(max_rounded_px_count)
            {
                *fista = mul_add!(factor, fdata - *fista, *fista);
            }
            std::mem::swap(&mut aux.fdata, &mut aux.fista);
        });

        term = next_term;

        // Take a step
        step::solver_step(
            &step::StepParams {
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                nchannel,
                step_size: radius / (1.0 + usize_to_f32(iterations)).sqrt(),
                weight,
            },
            &coefs,
            &mut auxs,
            &pweight,
        );
    }

    auxs.into_par_iter().map(|aux| aux.fdata).collect()
}
