use std::{
    ops::{Div, Mul, Sub},
    simd::f32x8,
};

use rayon::prelude::*;

use crate::utils::{
    auxiliary::Aux,
    coef::Coef,
    macros::mul_add,
    traits::{FromSlice, WriteTo},
};

/// One solver step shared by the SIMD pipelines: DCT-distance gradient, TV +
/// TGV regularization, normalized descent, then projection.
///
/// `prob_fn`/`proj_fn` are width-agnostic (`&[f32]`); `tv_fn`/`tv2_fn` are
/// closures so callers can pass their width tiling (uniform x8 for benchmarks,
/// adaptive 64/32/16/8 for the production pipeline). Scalar keeps its own
/// `compute_step` as reference.
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
    // DCT coefficient distance gradient
    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        aux.obj_gradient.fill(0.0);
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
            // Normalized gradient descent, 8-wide. `max_count` is a multiple of
            // 8 (JPEG rounds dimensions up to the block grid), so the step_by(8)
            // chunks are always exact.
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
