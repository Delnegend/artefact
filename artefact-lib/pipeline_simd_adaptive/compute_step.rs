use std::ops::{Div, Mul, Sub};

use rayon::prelude::*;

use super::{
    adaptive_width::AdaptiveWidth, coef::SIMDAdaptiveCoef, compute_projection::compute_projection,
    compute_step_prob::compute_step_prob, compute_step_tv::compute_step_tv,
    compute_step_tv2::compute_step_tv2,
};
use crate::{
    pipeline_simd_8::f32x8,
    utils::{
        aux::Aux,
        macros::mul_add,
        traits::{FromSlice, WriteTo},
    },
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
    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        aux.obj_gradient = vec![0.0; max_rounded_px_count];

        // DCT coefficient distance
        if pweight[c] != 0.0 {
            compute_step_prob(
                max_rounded_px_w,
                max_rounded_px_h,
                pweight[c] * 2.0 * 255.0 * 2.0_f32.sqrt(),
                &coefs[c],
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    });

    // TV computation
    compute_step_tv(
        max_rounded_px_w,
        max_rounded_px_h,
        nchannel,
        auxs,
        adaptive_widths,
    );

    // TGV second order
    compute_step_tv2(
        max_rounded_px_w,
        max_rounded_px_h,
        nchannel,
        auxs,
        weight / 2.0_f32.sqrt(),
        adaptive_widths,
    );

    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        // ===== Performs a gradient descent step in the direction of the
        // objective gradient with a specified step size. The gradient is
        // normalized before applying the step. =====

        // Calculate Euclidean norm of the objective gradient
        let norm = aux
            .obj_gradient
            .iter()
            .fold(0.0, |acc, &x| mul_add!(x, x, acc))
            .sqrt();

        // Only update if gradient norm is non-zero
        if norm != 0.0 {
            for i in (0..max_rounded_px_count).step_by(8) {
                let target = &mut aux.fdata[i..i + 8];

                let update = f32x8::from_slc(&aux.obj_gradient[i..i + 8])
                    .div(f32x8::splat(norm))
                    .mul(f32x8::splat(step_size));

                f32x8::from_slc(target).sub(update).write_to(target);
            }
        }

        // ===== Project onto DCT basis =====
        compute_projection(max_rounded_px_w, max_rounded_px_h, aux, &coefs[c]);
    });
}
