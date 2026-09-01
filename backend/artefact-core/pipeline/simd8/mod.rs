mod coef;
mod compute_projection;
mod compute_step_prob;
mod compute_step_tv;
mod compute_step_tv2;

pub use std::simd::f32x8;

pub use compute_step_tv::compute_step_tv;

use rayon::prelude::*;

use crate::{
    jpeg::Coefficient,
    pipeline::simd8::{
        compute_projection::compute_projection, compute_step_prob::compute_step_prob,
        compute_step_tv::compute_step_tv as compute_step_tv_fn, compute_step_tv2::compute_step_tv2,
    },
    utils::{fista, step},
};
use coef::SIMD8Coef;

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
    let radius = fista::radius(max_rounded_px_count);
    fista::fista_loop(
        fista::init_auxs(
            max_rounded_px_w,
            max_rounded_px_h,
            max_rounded_px_count,
            &coefs,
        ),
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
                compute_step_tv_fn,
                compute_step_tv2,
                compute_projection,
            );
        },
    )
}
