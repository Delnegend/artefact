use std::{
    ops::Div,
    simd::{Simd, StdFloat, cmp::SimdPartialEq},
};

use super::{
    adaptive_width::AdaptiveWidth,
    traits::{AddSlice, WriteTo},
};
use crate::utils::auxiliary::Aux;

/// First-order TV gradient: each run in `adaptive_widths` is processed with the
/// matching `Simd<f32, N>` lane width.
pub fn compute_step_tv(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    adaptive_widths: &[AdaptiveWidth],
) {
    for curr_row in 0..max_rounded_px_h {
        for adaptive_width in adaptive_widths {
            match adaptive_width {
                AdaptiveWidth::X8(x) => tv_inner::<8>(
                    max_rounded_px_w,
                    max_rounded_px_h,
                    nchannel,
                    auxs,
                    *x,
                    curr_row,
                ),
                AdaptiveWidth::X16(x) => tv_inner::<16>(
                    max_rounded_px_w,
                    max_rounded_px_h,
                    nchannel,
                    auxs,
                    *x,
                    curr_row,
                ),
                AdaptiveWidth::X32(x) => tv_inner::<32>(
                    max_rounded_px_w,
                    max_rounded_px_h,
                    nchannel,
                    auxs,
                    *x,
                    curr_row,
                ),
                AdaptiveWidth::X64(x) => tv_inner::<64>(
                    max_rounded_px_w,
                    max_rounded_px_h,
                    nchannel,
                    auxs,
                    *x,
                    curr_row,
                ),
            }
        }
    }
}

fn tv_inner<const N: usize>(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_px_idx: u32,
    curr_row: u32,
) {
    // A run is N pixels wide but the stencil only overlaps by 8, hence the
    // padding when indexing a run's neighbourhood.
    let pad = N - 8;
    let px_idx_start_of_group = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx as usize + 8 + pad == max_rounded_px_w as usize;
    let group_at_bottom_edge = curr_row + 1 == max_rounded_px_h;

    let mut g_xs = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_ys = [Simd::<f32, N>::splat(0.0); 3];

    // compute forward differences
    for c in 0..nchannel {
        let aux = &auxs[c];

        // forward difference x
        g_xs[c] = if group_at_right_edge {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 6 + pad;
            let curr_group = Simd::<f32, N>::load_or_default(&aux.fdata[a..=b]);

            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + 7 + pad;
            let shift_right_1px_group = Simd::<f32, N>::load_or_default(&aux.fdata[a..=b]);

            shift_right_1px_group - curr_group
        } else {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.fdata[a..=b]);

            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + 8 + pad;
            let shift_right_1px_group = Simd::<f32, N>::from_slice(&aux.fdata[a..=b]);

            shift_right_1px_group - curr_group
        };

        // forward difference y
        if !group_at_bottom_edge {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7 + pad;
            let curr_group = Simd::<f32, N>::from_slice(&aux.fdata[a..=b]);

            let a = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let shift_down_1px_group = Simd::<f32, N>::from_slice(&aux.fdata[a..=b]);

            g_ys[c] = shift_down_1px_group - curr_group;
        }
    }

    // compute gradient normalization
    let alpha = Simd::<f32, N>::splat(1.0 / (nchannel as f32).sqrt());
    let g_norm = (0..nchannel)
        .map(|c| g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c])
        .fold(Simd::<f32, N>::splat(0.0), |acc, x| acc + x)
        .sqrt();
    let mask = g_norm.simd_ne(Simd::<f32, N>::splat(0.0));

    for c in 0..nchannel {
        // ===== compute derivatives =====
        let aux = &mut auxs[c];

        {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7 + pad;
            let target = &mut aux.obj_gradient[a..=b];

            (alpha * -(g_xs[c] + g_ys[c]))
                .div(g_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        {
            if group_at_right_edge {
                let a = px_idx_start_of_group + 1;
                let b = px_idx_start_of_group + 7 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * g_xs[c])
                    .div(g_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let a = px_idx_start_of_group + 1;
                let b = px_idx_start_of_group + 8 + pad;
                let target = &mut aux.obj_gradient[a..=b];

                (alpha * g_xs[c])
                    .div(g_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // for shifted_down_1px_group aka group below the current group
        if !group_at_bottom_edge {
            let a = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let b = a + 7 + pad;
            let target = &mut aux.obj_gradient[a..=b];

            (alpha * g_ys[c])
                .div(g_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // ===== store for use in tv2 =====
        let a = px_idx_start_of_group;
        let b = px_idx_start_of_group + 7 + pad;

        g_xs[c].write_to(&mut auxs[c].pixel_diff.x[a..=b]);
        if !group_at_bottom_edge {
            g_ys[c].write_to(&mut auxs[c].pixel_diff.y[a..=b]);
        }
    }
}
