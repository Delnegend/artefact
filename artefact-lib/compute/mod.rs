mod aux;
mod compute_projection;
mod compute_step;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

use rayon::prelude::*;

use crate::{
    compute::{aux::Aux, compute_projection::compute_projection, compute_step::compute_step},
    jpeg::Coefficient,
};

pub fn compute(
    nchannel: usize,
    coefs: &mut [Coefficient],
    weight: f32,
    pweight: [f32; 3],
    iterations: u32,
) {
    let max_rounded_px_w = coefs[0].rounded_px_w;
    let max_rounded_px_h = coefs[0].rounded_px_h;
    let max_rounded_px_count = (max_rounded_px_w * max_rounded_px_h) as usize;

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
    for _i in 0..iterations {
        // FISTA update
        let next_term = (1.0 + (1.0 + 4.0 * term.powi(2)).sqrt()) / 2.0;
        let factor = (term - 1.0) / next_term;

        // Update all channels in parallel
        auxs.par_iter_mut().for_each(|aux| {
            for i in 0..max_rounded_px_count {
                aux.fista[i] = aux.fdata[i] + factor * (aux.fdata[i] - aux.fista[i]);
            }
            std::mem::swap(&mut aux.fdata, &mut aux.fista);
        });

        term = next_term;

        // // Take a step
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

        // Project onto DCT basis
        auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
            compute_projection(max_rounded_px_w, max_rounded_px_h, aux, &coefs[c]);
        });
    }

    // Update coefficients with results
    for c in 0..nchannel {
        coefs[c].rounded_px_w = max_rounded_px_w;
        coefs[c].rounded_px_h = max_rounded_px_h;
        coefs[c].image_data = auxs[c].fdata.clone(); // TODO: might need a Rc/Arc
    }
}
