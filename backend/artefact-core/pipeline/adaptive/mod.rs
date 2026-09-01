mod adaptive_width;
mod coef;
mod compute_projection;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

use rayon::prelude::*;

use crate::{
    jpeg::Coefficient,
    pipeline::adaptive::{
        compute_projection::compute_projection, compute_step_prob::compute_step_prob,
        compute_step_tv::compute_step_tv, compute_step_tv2::compute_step_tv2,
    },
    utils::{fista, step},
};
use adaptive_width::get_adaptive_widths;
use coef::SIMDAdaptiveCoef;

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
    let coefs = coefs
        .into_par_iter()
        .map(SIMDAdaptiveCoef::from)
        .collect::<Vec<_>>();
    let auxs = fista::init_auxs(
        max_rounded_px_w,
        max_rounded_px_h,
        max_rounded_px_count,
        &coefs,
    );
    let radius = fista::radius(max_rounded_px_count);
    let adaptive_widths = get_adaptive_widths(max_rounded_px_w);
    fista::fista_loop(
        auxs,
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
                |w, h, nch, auxs| compute_step_tv(w, h, nch, auxs, &adaptive_widths),
                |w, h, nch, auxs, alpha| compute_step_tv2(w, h, nch, auxs, alpha, &adaptive_widths),
                compute_projection,
            );
        },
    )
}
