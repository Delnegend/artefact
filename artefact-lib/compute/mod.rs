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

#[cfg(feature = "simd")]
pub mod adaptive_simd {
    #[derive(Debug, Clone, Copy)]
    pub enum AdaptiveWidth {
        X8(u32),
        X16(u32),
        X32(u32),
        X64(u32),
    }

    /// Returns the indexes of the current row in the image, wrapped inside a
    /// `GroupWidth` enum to indicate how many pixels can be processed at once.
    pub fn get_adaptive_widths(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
        let (mut tmp, mut idx) = (vec![], 0);
        loop {
            match idx {
                x if x + 64 <= max_rounded_px_w => {
                    tmp.push(AdaptiveWidth::X64(idx));
                    idx += 64;
                }
                x if x + 32 <= max_rounded_px_w => {
                    tmp.push(AdaptiveWidth::X32(idx));
                    idx += 32;
                }
                x if x + 16 <= max_rounded_px_w => {
                    tmp.push(AdaptiveWidth::X16(idx));
                    idx += 16;
                }
                x if x + 8 <= max_rounded_px_w => {
                    tmp.push(AdaptiveWidth::X8(idx));
                    idx += 8;
                }
                _ => break,
            }
        }
        tmp
    }
}

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

    let adaptive_widths = adaptive_simd::get_adaptive_widths(max_rounded_px_w);

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
            coefs,
            &mut auxs,
            radius / (1.0 + iterations as f32).sqrt(),
            weight,
            &pweight,
            &adaptive_widths,
        );
    }

    // Update coefficients with results
    for c in 0..nchannel {
        coefs[c].rounded_px_w = max_rounded_px_w;
        coefs[c].rounded_px_h = max_rounded_px_h;
        coefs[c].image_data = std::mem::take(&mut auxs[c].fdata);
    }
}
