use std::simd::{cmp::SimdPartialEq, f32x8, StdFloat};

use crate::compute::{aux::Aux, simd_std::f32x8};

/// uses [`f32x8`] from standard library instead of `wide`
///
/// [`f32x8`]: std::simd::f32x8
#[allow(unused)]
pub fn compute_step_tv_simd_std(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let alpha = f32x8::splat(1.0 / (nchannel as f32).sqrt());

    for curr_row in 0..max_rounded_px_h {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            compute_step_tv_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                curr_row_px_idx,
                curr_row,
                alpha,
            );
        }
    }
}

fn compute_step_tv_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_px_idx: u32,
    curr_row: u32,
    alpha: f32x8,
) {
    // a "group" = 8 consecutive pixels horizontally

    let px_idx_start_of_group = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx + 8 == max_rounded_px_w;
    let group_at_bottom_edge = curr_row + 1 == max_rounded_px_h;

    let mut g_xs = [f32x8!(); 3];
    let mut g_ys = [f32x8!(); 3];

    // compute forward differences
    for c in 0..nchannel {
        let aux = &auxs[c];

        // forward difference x
        g_xs[c] = if group_at_right_edge {
            // only handle 7 consecutive pixels because the last one is at the
            // edge, and there's no more pixel to the right for us to calculate
            // the difference with

            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 6;
            let curr_group = f32x8!(shorter: &aux.fdata[a..=b]);

            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + 7;
            let shift_right_1px_group = f32x8!(shorter: &aux.fdata[a..=b]);

            shift_right_1px_group - curr_group
        } else {
            // 8 pixels

            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7;
            let curr_group = f32x8!(&aux.fdata[a..=b]);

            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + 8;
            let shift_right_1px_group = f32x8!(&aux.fdata[a..=b]);

            shift_right_1px_group - curr_group
        };

        // forward difference y
        if !group_at_bottom_edge {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7;
            let curr_group = f32x8!(&aux.fdata[a..=b]);

            let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let shift_down_1px_group = f32x8!(&aux.fdata[i..=i + 7]);

            g_ys[c] = shift_down_1px_group - curr_group
        };
    }

    // compute gradient normalization
    let g_norm = (0..nchannel)
        .map(|c| g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c])
        .fold(f32x8!(), |acc, x| acc + x)
        .sqrt();
    let mask = g_norm.simd_ne(f32x8!());

    // compute derivatives
    for c in 0..nchannel {
        let aux = &mut auxs[c];

        '_for_current_group: {
            let a = px_idx_start_of_group;
            let b = px_idx_start_of_group + 7;

            let target = &mut aux.obj_gradient[a..=b];
            let result = f32x8!(target) + alpha * -(g_xs[c] + g_ys[c]) / g_norm;
            result.store_select(target, mask);
        }

        '_for_shifted_right_1px_group: {
            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + if group_at_right_edge { 7 } else { 8 };

            let target = &mut aux.obj_gradient[a..=b];
            let result = f32x8!(shorter: &target[..]) + alpha * g_xs[c] / g_norm;

            result.store_select(target, mask);
        }

        // for shifted_down_1px_group aka group below the current group
        if !group_at_bottom_edge {
            let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = &mut aux.obj_gradient[i..=i + 7];
            let result = f32x8!(target) + alpha * g_ys[c] / g_norm;

            result.store_select(target, mask);
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        let a = px_idx_start_of_group;
        let b = px_idx_start_of_group + 7;

        g_xs[c].copy_to_slice(&mut auxs[c].pixel_diff.x[a..=b]);
        if !group_at_bottom_edge {
            g_ys[c].copy_to_slice(&mut auxs[c].pixel_diff.y[a..=b]);
        }
    }
}
