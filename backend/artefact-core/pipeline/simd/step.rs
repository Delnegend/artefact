use std::{
    ops::{Div, Mul, Sub},
    simd::f32x8,
};

use rayon::prelude::*;

use super::{coef::Coef, traits::WriteTo};
use crate::utils::{auxiliary::Aux, macros::mul_add};

/// One iteration of the projected subgradient solver shared by the SIMD
/// pipelines: the Discrete Cosine Transform (DCT) data-fidelity gradient, the
/// Total Variation (TV) and second-order Total Generalized Variation (TGV)
/// regularization gradients, a normalized descent step, then the projection
/// back onto the feasible set.
///
/// `dct_gradient_fn`/`project_fn` are width-agnostic (`&[f32]`); `tv_gradient_fn`/`tgv_gradient_fn` are
/// closures so callers can pass their width tiling (uniform x8 for benchmarks,
/// adaptive 64/32/16/8 for the production pipeline). Scalar keeps its own
/// `solver_step` as reference.
#[allow(clippy::too_many_arguments)]
pub fn solver_step<C, ProbFn, TvFn, Tv2Fn, ProjFn>(
    max_w: u32,
    max_h: u32,
    max_count: usize,
    nchannel: usize,
    coefs: &[C],
    auxs: &mut [Aux],
    norm: &mut [f32],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
    dct_gradient_fn: ProbFn,
    mut tv_gradient_fn: TvFn,
    mut tgv_gradient_fn: Tv2Fn,
    project_fn: ProjFn,
) where
    C: Coef + Sync,
    ProbFn: Fn(u32, u32, f32, &C, &[f32], &mut [f32]) + Sync,
    TvFn: FnMut(u32, u32, usize, &mut [Aux], &mut [f32]),
    Tv2Fn: FnMut(u32, u32, usize, &mut [Aux], f32, &mut [f32]),
    ProjFn: Fn(u32, u32, &mut Aux, &C) + Sync,
{
    // TV initialises `obj_gradient` (writes it), so no zero-fill is needed.
    tv_gradient_fn(max_w, max_h, nchannel, auxs, norm);

    // DCT coefficient distance gradient (adds on top of TV)
    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        if pweight[c] != 0.0 {
            dct_gradient_fn(
                max_w,
                max_h,
                pweight[c] * 2.0 * 255.0 * 2.0_f32.sqrt(),
                &coefs[c],
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    });

    tgv_gradient_fn(max_w, max_h, nchannel, auxs, weight / 2.0_f32.sqrt(), norm);

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
                let update = f32x8::from_slice(&aux.obj_gradient[i..i + 8])
                    .div(f32x8::splat(norm))
                    .mul(f32x8::splat(step_size));
                f32x8::from_slice(target).sub(update).write_to(target);
            }
        }

        project_fn(max_w, max_h, aux, &coefs[c]);
    });
}
