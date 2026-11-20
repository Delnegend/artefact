use std::{
    ops::{Add, Mul, Neg, Sub},
    simd::{Simd, StdFloat, f32x8},
};

use rayon::prelude::*;

use super::{
    adaptive_width::{AdaptiveWidth, dispatch_run},
    run::{load, load_or_zero, load_shifted_right},
    traits::{SafeDiv, WriteTo},
};
use crate::utils::auxiliary::Aux;

/// First-order Total Variation (TV) gradient, parallel over rows.
///
/// Three race-free passes:
/// 1. per-channel forward differences into `pixel_diff`;
/// 2. the cross-channel per-pixel norm into `norm`;
/// 3. a gather of each pixel's own + left + up contributions into `obj_gradient`.
pub fn tv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    adaptive_widths: &[AdaptiveWidth],
    norm: &mut [f32],
) {
    // 1. forward differences
    auxs.par_iter_mut().for_each(|aux| {
        let Aux {
            pixel_diff, fdata, ..
        } = aux;
        let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
        if w == 0 {
            return;
        }
        pixel_diff
            .x
            .par_chunks_mut(w)
            .zip(pixel_diff.y.par_chunks_mut(w))
            .enumerate()
            .for_each(|(y, (px, py))| {
                if let Ok(y_u32) = u32::try_from(y) {
                    for &run in adaptive_widths {
                        dispatch_run!(
                            run,
                            compute_g_run,
                            fdata,
                            max_rounded_px_w,
                            max_rounded_px_h,
                            px,
                            py;
                            y_u32
                        );
                    }
                }
            });
    });

    // 2. cross-channel norm
    compute_norm(auxs, norm, max_rounded_px_w);

    // 3. gather the contributions
    auxs.par_iter_mut().for_each(|aux| {
        let Aux {
            obj_gradient,
            pixel_diff,
            ..
        } = aux;
        let gx = &pixel_diff.x;
        let gy = &pixel_diff.y;
        let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
        if w == 0 {
            return;
        }
        obj_gradient
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, obj_row)| {
                if let Ok(y_u32) = u32::try_from(y) {
                    let ctx = GatherCtx {
                        gx,
                        gy,
                        norm,
                        max_rounded_px_w,
                        nchannel,
                        y: y_u32,
                    };
                    for &run in adaptive_widths {
                        dispatch_run!(run, gather_run, &ctx; obj_row);
                    }
                }
            });
    });
}

/// Forward differences for one run: `pixel_diff.x` gets `f(x+1)-f(x)`,
/// `pixel_diff.y` gets `f(y+1)-f(y)`.
fn compute_g_run<const N: usize>(
    fdata: &[f32],
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    px: &mut [f32],
    py: &mut [f32],
    x0: u32,
    y: u32,
) {
    let pad = N.saturating_sub(8);
    let n_minus_1 = N.saturating_sub(1);
    let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
    let h = usize::try_from(max_rounded_px_h).unwrap_or(0);
    let x = usize::try_from(x0).unwrap_or(0);
    let y_usize = usize::try_from(y).unwrap_or(0);
    let start = y_usize.saturating_mul(w).saturating_add(x);
    let at_right_edge = x.saturating_add(8).saturating_add(pad) == w;
    let at_bottom_edge = y_usize.saturating_add(1) == h;

    let gx = if at_right_edge {
        load_or_zero::<N>(fdata, start.saturating_add(1), n_minus_1)
            .sub(load_or_zero::<N>(fdata, start, n_minus_1))
    } else {
        load::<N>(fdata, start.saturating_add(1)).sub(load::<N>(fdata, start))
    };
    let gy = if at_bottom_edge {
        Simd::<f32, N>::splat(0.0)
    } else {
        load::<N>(fdata, start.saturating_add(w)).sub(load::<N>(fdata, start))
    };

    if let Some(dst) = px.get_mut(x..x.saturating_add(N)) {
        gx.write_to(dst);
    }
    if let Some(dst) = py.get_mut(x..x.saturating_add(N)) {
        gy.write_to(dst);
    }
}

/// Cross-channel norm `sqrt(sum_c gx^2 + gy^2)` for every pixel.
fn compute_norm(auxs: &[Aux], norm: &mut [f32], max_rounded_px_w: u32) {
    let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
    if w == 0 {
        return;
    }
    norm.par_chunks_mut(w).enumerate().for_each(|(y, n_row)| {
        let base = y.saturating_mul(w);
        for (chunk_idx, n_chunk) in n_row.as_chunks_mut::<8>().0.iter_mut().enumerate() {
            let offset = base.saturating_add(chunk_idx.saturating_mul(8));
            let mut acc = f32x8::splat(0.0);
            for aux in auxs {
                if let (Some(gx_slice), Some(gy_slice)) = (
                    aux.pixel_diff.x.get(offset..offset.saturating_add(8)),
                    aux.pixel_diff.y.get(offset..offset.saturating_add(8)),
                ) {
                    let gx = f32x8::from_slice(gx_slice);
                    let gy = f32x8::from_slice(gy_slice);
                    acc = acc.add(gx.mul(gx).add(gy.mul(gy)));
                }
            }
            acc.sqrt().write_to(n_chunk);
        }
    });
}

/// Gather one run's TV gradient: own `-(gx+gy)/n` plus the left pixel's `gx/n`
/// and the pixel above's `gy/n`.
struct GatherCtx<'a> {
    gx: &'a [f32],
    gy: &'a [f32],
    norm: &'a [f32],
    max_rounded_px_w: u32,
    nchannel: usize,
    y: u32,
}

fn gather_run<const N: usize>(ctx: &GatherCtx<'_>, x0: u32, obj: &mut [f32]) {
    let &GatherCtx {
        gx,
        gy,
        norm,
        max_rounded_px_w,
        nchannel,
        y,
    } = ctx;
    let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
    let x = usize::try_from(x0).unwrap_or(0);
    let row = usize::try_from(y).unwrap_or(0).saturating_mul(w);
    let row_x = row.saturating_add(x);
    let n_minus_1 = N.saturating_sub(1);

    let own_x = load::<N>(gx, row_x);
    let own_y = load::<N>(gy, row_x);
    let own_n = load::<N>(norm, row_x);

    let left_x = if x == 0 {
        load_shifted_right::<N>(gx, row, n_minus_1)
    } else {
        load::<N>(gx, row_x.saturating_sub(1))
    };
    let left_n = if x == 0 {
        load_shifted_right::<N>(norm, row, n_minus_1)
    } else {
        load::<N>(norm, row_x.saturating_sub(1))
    };

    let (up_y, up_n) = if y == 0 {
        (Simd::<f32, N>::splat(0.0), Simd::<f32, N>::splat(0.0))
    } else {
        let up = row.saturating_sub(w).saturating_add(x);
        (load::<N>(gy, up), load::<N>(norm, up))
    };
    let nch_f32 = f32::from(u16::try_from(nchannel).unwrap_or(u16::MAX));
    let alpha = Simd::<f32, N>::splat(1.0 / nch_f32.sqrt());
    let value = alpha.mul(
        own_x
            .add(own_y)
            .safe_div(own_n)
            .neg()
            .add(left_x.safe_div(left_n))
            .add(up_y.safe_div(up_n)),
    );

    // The TV pass initialises `obj_gradient`, so this is a plain store.
    if let Some(dst) = obj.get_mut(x..x.saturating_add(N)) {
        value.write_to(dst);
    }
}
