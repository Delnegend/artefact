use std::simd::{Simd, StdFloat, f32x8};

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
        let w = max_rounded_px_w as usize;
        pixel_diff
            .x
            .par_chunks_mut(w)
            .zip(pixel_diff.y.par_chunks_mut(w))
            .enumerate()
            .for_each(|(y, (px, py))| {
                for &run in adaptive_widths {
                    dispatch_run!(
                        run,
                        compute_g_run,
                        fdata,
                        max_rounded_px_w,
                        max_rounded_px_h,
                        px,
                        py;
                        y as u32
                    );
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
        let w = max_rounded_px_w as usize;
        obj_gradient
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, obj_row)| {
                for &run in adaptive_widths {
                    dispatch_run!(
                        run,
                        gather_run,
                        gx,
                        gy,
                        norm,
                        obj_row,
                        max_rounded_px_w,
                        max_rounded_px_h,
                        nchannel;
                        y as u32
                    );
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
    let pad = N - 8;
    let w = max_rounded_px_w as usize;
    let x = x0 as usize;
    let start = y as usize * w + x;
    let at_right_edge = x + 8 + pad == w;
    let at_bottom_edge = y as usize + 1 == max_rounded_px_h as usize;

    let gx = if at_right_edge {
        load_or_zero::<N>(fdata, start + 1, N - 1) - load_or_zero::<N>(fdata, start, N - 1)
    } else {
        load::<N>(fdata, start + 1) - load::<N>(fdata, start)
    };
    let gy = if at_bottom_edge {
        Simd::<f32, N>::splat(0.0)
    } else {
        load::<N>(fdata, start + w) - load::<N>(fdata, start)
    };

    gx.write_to(&mut px[x..x + N]);
    gy.write_to(&mut py[x..x + N]);
}

/// Cross-channel norm `sqrt(sum_c gx^2 + gy^2)` for every pixel.
fn compute_norm(auxs: &[Aux], norm: &mut [f32], max_rounded_px_w: u32) {
    let w = max_rounded_px_w as usize;
    norm.par_chunks_mut(w).enumerate().for_each(|(y, n_row)| {
        let base = y * w;
        for x in (0..w).step_by(8) {
            let mut acc = f32x8::splat(0.0);
            for aux in auxs {
                let gx = f32x8::from_slice(&aux.pixel_diff.x[base + x..base + x + 8]);
                let gy = f32x8::from_slice(&aux.pixel_diff.y[base + x..base + x + 8]);
                acc += gx * gx + gy * gy;
            }
            acc.sqrt().write_to(&mut n_row[x..x + 8]);
        }
    });
}

/// Gather one run's TV gradient: own `-(gx+gy)/n` plus the left pixel's `gx/n`
/// and the pixel above's `gy/n`.
#[allow(clippy::too_many_arguments)]
fn gather_run<const N: usize>(
    gx: &[f32],
    gy: &[f32],
    norm: &[f32],
    obj: &mut [f32],
    max_rounded_px_w: u32,
    _max_rounded_px_h: u32,
    nchannel: usize,
    x0: u32,
    y: u32,
) {
    let w = max_rounded_px_w as usize;
    let x = x0 as usize;
    let row = y as usize * w;

    let own_x = load::<N>(gx, row + x);
    let own_y = load::<N>(gy, row + x);
    let own_n = load::<N>(norm, row + x);

    let left_x = if x == 0 {
        load_shifted_right::<N>(gx, row, N - 1)
    } else {
        load::<N>(gx, row + x - 1)
    };
    let left_n = if x == 0 {
        load_shifted_right::<N>(norm, row, N - 1)
    } else {
        load::<N>(norm, row + x - 1)
    };

    let (up_y, up_n) = if y == 0 {
        (Simd::<f32, N>::splat(0.0), Simd::<f32, N>::splat(0.0))
    } else {
        (load::<N>(gy, row - w + x), load::<N>(norm, row - w + x))
    };
    let alpha = Simd::<f32, N>::splat(1.0 / (nchannel as f32).sqrt());
    let value =
        alpha * (-(own_x + own_y).safe_div(own_n) + left_x.safe_div(left_n) + up_y.safe_div(up_n));

    // The TV pass initialises `obj_gradient`, so this is a plain store.
    value.write_to(&mut obj[x..x + N]);
}
