use std::{
    ops::Div,
    simd::{Simd, StdFloat, cmp::SimdPartialEq},
};

use super::{
    adaptive_width::{AdaptiveWidth, dispatch_width},
    traits::{AddSlice, FromSlice, SafeDiv, WriteTo},
};
use crate::utils::auxiliary::Aux;

/// Second-order (TGV) TV gradient: each run in `adaptive_widths` is processed
/// with the matching `Simd<f32, N>` lane width.
pub fn compute_step_tv2(
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
            dispatch_width!(
                adaptive_width,
                tv2_inner,
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
fn tv2_inner<const N: usize>(
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

    let curr_group_idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_top_edge = curr_row == 0;
    let group_at_left_edge = curr_row_px_idx == 0;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;
    let group_at_right_edge = curr_row_px_idx as usize + 8 + pad >= max_rounded_px_w as usize;

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // backward difference x
        let g_yx = if group_at_left_edge {
            let a = curr_group_idx + 1;
            let b = curr_group_idx + 7 + pad;
            let curr_group =
                Simd::<f32, N>::from_range_slice(&aux.pixel_diff.y[a..=b], 1..=7 + pad);

            let a = curr_group_idx;
            let b = curr_group_idx + 6 + pad;
            let shift_left_1px_group =
                Simd::<f32, N>::from_range_slice(&aux.pixel_diff.y[a..=b], 1..=7 + pad);

            curr_group - shift_left_1px_group
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.y[a..=b]);

            let a = curr_group_idx - 1;
            let b = curr_group_idx + 6 + pad;
            let shift_left_1px_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.y[a..=b]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        let g_xy = if group_at_top_edge {
            Simd::<f32, N>::splat(0.0)
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.x[a..=b]);

            let a = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let shift_up_1px_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.x[a..=b]);

            curr_group - shift_up_1px_group
        };

        // backward difference x
        g_xxs[c] = if group_at_left_edge {
            let a = curr_group_idx + 1;
            let b = curr_group_idx + 7 + pad;
            let curr_group =
                Simd::<f32, N>::from_range_slice(&aux.pixel_diff.x[a..=b], 1..=7 + pad);

            let a = curr_group_idx;
            let b = curr_group_idx + 6 + pad;
            let shift_left_1px_group =
                Simd::<f32, N>::from_range_slice(&aux.pixel_diff.x[a..=b], 1..=7 + pad);

            curr_group - shift_left_1px_group
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.x[a..=b]);

            let a = curr_group_idx - 1;
            let b = curr_group_idx + 6 + pad;
            let shift_left_1px_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.x[a..=b]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        g_yys[c] = if group_at_top_edge {
            Simd::<f32, N>::splat(0.0)
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.y[a..=b]);

            let a = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let shift_up_1px_group = Simd::<f32, N>::from_slice(&aux.pixel_diff.y[a..=b]);

            curr_group - shift_up_1px_group
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
            let a = curr_group_idx;
            let b = curr_group_idx + 7 + pad;
            let target = &mut aux.obj_gradient[a..=b];

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

                let a = curr_group_idx;
                let b = curr_group_idx + 6 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * (g_xy_sym + g_xx))
                    .safe_div(g2_norm)
                    .add_range_slice(target, 1..=7 + pad)
                    .write_partial_to(target, 1..=7 + pad);
            } else {
                let a = curr_group_idx - 1;
                let b = curr_group_idx + 6 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        {
            if group_at_right_edge {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 7 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 8 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * (g_xy_sym + g_xx))
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // for shifted up 1px group | group above the current group
        if !group_at_top_edge {
            let a = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let target = &mut aux.obj_gradient[a..=b];

            (alpha * (g_yy + g_xy_sym))
                .div(g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // for shifted down 1px group | group below the current group
        if !group_at_bottom_edge {
            let a = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let target = &mut aux.obj_gradient[a..=b];

            (alpha * (g_yy + g_xy_sym))
                .div(g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // for shift up right 1px group
        if !group_at_top_edge {
            if group_at_right_edge {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 7 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 8 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // for shift down left 1px group
        if !group_at_bottom_edge {
            if group_at_left_edge {
                let a = curr_group_idx;
                let b = curr_group_idx + 6 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * -g_xy_sym)
                    .safe_div(g2_norm)
                    .add_range_slice(target, 1..=7 + pad)
                    .write_partial_to(target, 1..=7 + pad);
            } else {
                let a = curr_group_idx - 1;
                let b = curr_group_idx + 6 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * -g_xy_sym)
                    .div(g2_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }
    }
}
