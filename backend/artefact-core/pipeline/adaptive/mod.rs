mod adaptive_width;
mod coef;
mod compute_projection;
mod compute_step;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

use rayon::prelude::*;

use crate::{jpeg::Coefficient, utils::fista};
use adaptive_width::get_adaptive_widths;
use coef::SIMDAdaptiveCoef;
use compute_step::compute_step;

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
            compute_step(
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
                nchannel,
                coefs,
                auxs,
                step_size,
                weight,
                &pweight,
                &adaptive_widths,
            );
        },
    )
}
