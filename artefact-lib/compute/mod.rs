mod aux;

#[cfg(not(feature = "simd"))]
mod scalar;
#[cfg(feature = "simd")]
mod simd;

use rayon::prelude::*;

use crate::{compute::aux::Aux, jpeg::Coefficient, utils::macros::mul_add};

#[cfg(not(feature = "simd"))]
use scalar::compute_step::compute_step;
#[cfg(feature = "simd")]
use simd::compute_step::compute_step;

#[allow(clippy::too_many_arguments)]
pub fn compute(
    nchannel: usize,
    coefs: &mut [Coefficient],
    weight: f32,
    pweight: [f32; 3],
    iterations: usize,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
) {
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
        let next_term = (1.0 + (1.0 + 4.0 * term.powi(2)).sqrt()) / 2.0;
        let factor = (term - 1.0) / next_term;

        auxs.par_iter_mut().for_each(|aux| {
            for i in 0..max_rounded_px_count {
                aux.fista[i] = aux.fdata[i] + factor * (aux.fdata[i] - aux.fista[i]);
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
            coefs,
            &mut auxs,
            radius / (1.0 + iterations as f32).sqrt(),
            weight,
            &pweight,
        );
    }

    // Update coefficients with results
    for c in 0..nchannel {
        coefs[c].rounded_px_w = max_rounded_px_w;
        coefs[c].rounded_px_h = max_rounded_px_h;
        coefs[c].image_data = std::mem::take(&mut auxs[c].fdata);
    }
}
