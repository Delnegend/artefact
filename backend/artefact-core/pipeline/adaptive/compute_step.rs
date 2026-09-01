use crate::{
    pipeline::adaptive::{
        adaptive_width::AdaptiveWidth, coef::SIMDAdaptiveCoef,
        compute_projection::compute_projection, compute_step_prob::compute_step_prob,
        compute_step_tv::compute_step_tv, compute_step_tv2::compute_step_tv2,
    },
    utils::{auxiliary::Aux, step},
};

#[allow(clippy::too_many_arguments)]
pub fn compute_step(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
    nchannel: usize,
    coefs: &[SIMDAdaptiveCoef],
    auxs: &mut [Aux],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
    adaptive_widths: &[AdaptiveWidth],
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
        |w, h, nch, auxs| compute_step_tv(w, h, nch, auxs, adaptive_widths),
        |w, h, nch, auxs, alpha| compute_step_tv2(w, h, nch, auxs, alpha, adaptive_widths),
        compute_projection,
    );
}
