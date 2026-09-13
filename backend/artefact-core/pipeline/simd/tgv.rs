use std::{
    ops::Div,
    simd::{Simd, StdFloat, cmp::SimdPartialEq},
};

use super::{
    adaptive_width::{AdaptiveWidth, dispatch_run},
    run::{add_shifted_left, load, load_shifted_right, run_mut},
    traits::{AddSlice, SafeDiv},
};
use crate::utils::auxiliary::Aux;

/// Second-order Total Generalized Variation (TGV) gradient: each run in
/// `adaptive_widths` is processed with the matching `Simd<f32, N>` lane width.
pub fn tgv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
    adaptive_widths: &[AdaptiveWidth],
) {
    let alpha = alpha / (nchannel as f32).sqrt();

    for curr_row in 0..max_rounded_px_h {
        for &adaptive_width in adaptive_widths {
            dispatch_run!(
                adaptive_width,
                tgv_run,
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                alpha;
                curr_row
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Second-order Total Generalized Variation (TGV) backward-difference gradient
/// for one run of `N` lanes.
fn tgv_run<const N: usize>(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
    curr_row_px_idx: u32,
    curr_row: u32,
) {
    // A run is N pixels wide but the stencil only overlaps by 8, hence the
    // padding when indexing a run's neighbourhood.
    let pad = N - 8;
    let mut g_xxs = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_yys = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_xy_syms = [Simd::<f32, N>::splat(0.0); 3];

    let idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_top_edge = curr_row == 0;
    let group_at_left_edge = curr_row_px_idx == 0;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;
    let group_at_right_edge = curr_row_px_idx as usize + 8 + pad >= max_rounded_px_w as usize;

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // backward difference x of `pixel_diff.y`
        let g_yx = if group_at_left_edge {
            load_shifted_right::<N>(&aux.pixel_diff.y, idx + 1, N - 1)
                - load_shifted_right::<N>(&aux.pixel_diff.y, idx, N - 1)
        } else {
            load::<N>(&aux.pixel_diff.y, idx) - load::<N>(&aux.pixel_diff.y, idx - 1)
        };

        // backward difference y of `pixel_diff.x`
        let g_xy = if group_at_top_edge {
            Simd::<f32, N>::splat(0.0)
        } else {
            let above = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            load::<N>(&aux.pixel_diff.x, idx) - load::<N>(&aux.pixel_diff.x, above)
        };

        // backward difference x of `pixel_diff.x`
        g_xxs[c] = if group_at_left_edge {
            load_shifted_right::<N>(&aux.pixel_diff.x, idx + 1, N - 1)
                - load_shifted_right::<N>(&aux.pixel_diff.x, idx, N - 1)
        } else {
            load::<N>(&aux.pixel_diff.x, idx) - load::<N>(&aux.pixel_diff.x, idx - 1)
        };

        // backward difference y of `pixel_diff.y`
        g_yys[c] = if group_at_top_edge {
            Simd::<f32, N>::splat(0.0)
        } else {
            let above = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            load::<N>(&aux.pixel_diff.y, idx) - load::<N>(&aux.pixel_diff.y, above)
        };

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / Simd::<f32, N>::splat(2.0);
    }

    // gradient normalization
    let alpha = Simd::<f32, N>::splat(alpha);
    let g2_norm = (0..nchannel)
        .map(|c| {
            g_xxs[c] * g_xxs[c]
                + Simd::<f32, N>::splat(2.0) * g_xy_syms[c] * g_xy_syms[c]
                + g_yys[c] * g_yys[c]
        })
        .fold(Simd::<f32, N>::splat(0.0), |acc, x| acc + x)
        .sqrt();
    let mask = g2_norm.simd_ne(Simd::<f32, N>::splat(0.0));

    // compute derivatives
    for c in 0..nchannel {
        let g_xx = g_xxs[c];
        let g_yy = g_yys[c];
        let g_xy_sym = g_xy_syms[c];
        let aux = &mut auxs[c];

        {
            let target = run_mut(&mut aux.obj_gradient, idx, N);

            (alpha
                * -(Simd::<f32, N>::splat(2.0) * g_xx
                    + Simd::<f32, N>::splat(2.0) * g_xy_sym
                    + Simd::<f32, N>::splat(2.0) * g_yy))
                .div(g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        {
            if group_at_left_edge {
                // ignore the first pixel in the group because it's out of bounds
                // [_] [0] [1] [2] [3] [4] [5] [6]

                let target = run_mut(&mut aux.obj_gradient, idx, N - 1);

                add_shifted_left::<N>(target, (alpha * (g_xy_sym + g_xx)).safe_div(g2_norm));
            } else {
                let target = run_mut(&mut aux.obj_gradient, idx - 1, N);

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        {
            if group_at_right_edge {
                let target = run_mut(&mut aux.obj_gradient, idx + 1, N - 1);

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let target = run_mut(&mut aux.obj_gradient, idx + 1, N);

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // for the group above the current group
        if !group_at_top_edge {
            let above = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let target = run_mut(&mut aux.obj_gradient, above, N);

            (alpha * (g_yy + g_xy_sym))
                .div(g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // for the group below the current group
        if !group_at_bottom_edge {
            let below = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let target = run_mut(&mut aux.obj_gradient, below, N);

            (alpha * (g_yy + g_xy_sym))
                .div(g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // up-right diagonal (x + 1, y - 1)
        if !group_at_top_edge {
            let base = idx + 1 - max_rounded_px_w as usize;
            if group_at_right_edge {
                // the last lane (x = w - 1) has no x + 1
                let target = run_mut(&mut aux.obj_gradient, base, N - 1);

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let target = run_mut(&mut aux.obj_gradient, base, N);

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // down-left diagonal (x - 1, y + 1)
        if !group_at_bottom_edge {
            if group_at_left_edge {
                // the first lane (x = 0) has no x - 1
                let target = run_mut(
                    &mut aux.obj_gradient,
                    idx + max_rounded_px_w as usize,
                    N - 1,
                );

                add_shifted_left::<N>(target, (alpha * -g_xy_sym).safe_div(g2_norm));
            } else {
                let target = run_mut(
                    &mut aux.obj_gradient,
                    idx + max_rounded_px_w as usize - 1,
                    N,
                );

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }
    }
}
