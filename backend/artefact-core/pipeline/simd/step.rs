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
pub struct StepCtx<'a, C> {
    pub max_w: u32,
    pub max_h: u32,
    pub max_count: usize,
    pub nchannel: usize,
    pub coefs: &'a [C],
    pub step_size: f32,
    pub weight: f32,
    pub pweight: &'a [f32; 3],
}

pub fn solver_step<C, ProbFn, TvFn, Tv2Fn, ProjFn>(
    ctx: &StepCtx<'_, C>,
    auxs: &mut [Aux],
    norm: &mut [f32],
    dct_gradient_fn: ProbFn,
    mut tv_gradient_fn: TvFn,
    mut second_order_fn: Tv2Fn,
    project_fn: ProjFn,
) where
    C: Coef + Sync,
    ProbFn: Fn(u32, u32, f32, &C, &[f32], &mut [f32]) + Sync,
    TvFn: FnMut(u32, u32, usize, &mut [Aux], &mut [f32]),
    Tv2Fn: FnMut(u32, u32, usize, &mut [Aux], f32, &mut [f32]),
    ProjFn: Fn(u32, u32, &mut Aux, &C) + Sync,
{
    let &StepCtx {
        max_w,
        max_h,
        max_count,
        nchannel,
        coefs,
        step_size,
        weight,
        pweight,
    } = ctx;
    // TV initialises `obj_gradient` (writes it), so no zero-fill is needed.
    tv_gradient_fn(max_w, max_h, nchannel, auxs, norm);

    // DCT coefficient distance gradient (adds on top of TV)
    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        if let (Some(&pw), Some(coef)) = (pweight.get(c), coefs.get(c))
            && pw != 0.0
        {
            dct_gradient_fn(
                max_w,
                max_h,
                pw * 2.0 * 255.0 * 2.0_f32.sqrt(),
                coef,
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    });

    second_order_fn(max_w, max_h, nchannel, auxs, weight / 2.0_f32.sqrt(), norm);

    auxs.par_iter_mut().enumerate().for_each(|(c, aux)| {
        let norm = aux
            .obj_gradient
            .iter()
            .fold(0.0, |acc, &x| mul_add!(x, x, acc))
            .sqrt();

        if norm != 0.0 {
            // Normalized gradient descent, 8-wide. `max_count` is a multiple of
            // 8 (JPEG rounds dimensions up to the block grid), so the chunks
            // are always exact.
            let norm_v = f32x8::splat(norm);
            let step_v = f32x8::splat(step_size);
            for (target, grad) in aux
                .fdata
                .as_chunks_mut::<8>()
                .0
                .iter_mut()
                .zip(aux.obj_gradient.as_chunks::<8>().0.iter())
                .take(max_count / 8)
            {
                let update = f32x8::from_slice(grad).div(norm_v).mul(step_v);
                f32x8::from_slice(target).sub(update).write_to(target);
            }
        }

        if let Some(coef) = coefs.get(c) {
            project_fn(max_w, max_h, aux, coef);
        }
    });
}
