use rayon::prelude::*;

use crate::utils::{auxiliary::Aux, coef::Coef, macros::mul_add};

/// Generic orchestration for simd pipelines (simd8 & adaptive).
/// Scalar keeps its own `compute_step` as reference.
///
/// `prob_fn` and `proj_fn` are width-specific (f32x8 vs f32x64),
/// `tv_fn`/`tv2_fn` are dispatched via closures to handle adaptive widths.

#[allow(clippy::too_many_arguments)]
pub fn step<C, ProbFn, TvFn, Tv2Fn, ProjFn>(
    max_w: u32,
    max_h: u32,
    max_count: usize,
    nchannel: usize,
    coefs: &[C],
    auxs: &mut [Aux],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
    prob_fn: ProbFn,
    mut tv_fn: TvFn,
    mut tv2_fn: Tv2Fn,
    proj_fn: ProjFn,
) where
    C: Coef + Sync,
    ProbFn: Fn(u32, u32, f32, &C, &[f32], &mut [f32]) + Sync,
    TvFn: FnMut(u32, u32, usize, &mut [Aux]),
    Tv2Fn: FnMut(u32, u32, usize, &mut [Aux], f32),
    ProjFn: Fn(u32, u32, &mut Aux, &C) + Sync,
{
    // DCT coefficient distance
    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        aux.obj_gradient = vec![0.0; max_count];
        if pweight[c] != 0.0 {
            prob_fn(
                max_w,
                max_h,
                pweight[c] * 2.0 * 255.0 * 2.0_f32.sqrt(),
                &coefs[c],
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    });

    tv_fn(max_w, max_h, nchannel, auxs);
    tv2_fn(max_w, max_h, nchannel, auxs, weight / 2.0_f32.sqrt());

    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        let norm = aux
            .obj_gradient
            .iter()
            .fold(0.0, |acc, &x| mul_add!(x, x, acc))
            .sqrt();
        if norm != 0.0 {
            // simd descent is width-specific — delegated to caller via closure
            // For simd8/adaptive we use f32x8 step_by(8) here, but to keep
            // this generic we call the provided proj_fn after descent.
            // Descent is done inline in the caller closure to avoid trait complexity.
            // This generic just handles norm; caller does the actual fdata update
            // before calling proj_fn if needed. To keep it simple, we do the
            // f32x8 descent here (common for both simd pipelines).
            use crate::utils::traits::{FromSlice, WriteTo};
            use std::ops::{Div, Mul, Sub};
            use std::simd::f32x8;
            for i in (0..max_count).step_by(8) {
                let target = &mut aux.fdata[i..i + 8];
                let update = f32x8::from_slc(&aux.obj_gradient[i..i + 8])
                    .div(f32x8::splat(norm))
                    .mul(f32x8::splat(step_size));
                f32x8::from_slc(target).sub(update).write_to(target);
            }
        }
        proj_fn(max_w, max_h, aux, &coefs[c]);
    });
}
