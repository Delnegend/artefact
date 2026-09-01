use crate::{
    pipeline::simd8::{
        coef::SIMD8Coef, compute_projection::compute_projection,
        compute_step_prob::compute_step_prob, compute_step_tv::compute_step_tv,
        compute_step_tv2::compute_step_tv2,
    },
    utils::{auxiliary::Aux, step},
};

#[allow(clippy::too_many_arguments)]
pub fn compute_step(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
    nchannel: usize,
    coefs: &[SIMD8Coef],
    auxs: &mut [Aux],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
) {
    step::step(
        max_rounded_px_w,
        max_rounded_px_h,
        max_rounded_px_count,
        nchannel,
        coefs,
        auxs,
        step_size,
        weight,
        pweight,
        compute_step_prob,
        compute_step_tv,
        compute_step_tv2,
        compute_projection,
    );
}
