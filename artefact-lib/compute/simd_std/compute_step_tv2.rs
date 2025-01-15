use std::simd::{cmp::SimdPartialEq, f32x8, StdFloat};

use crate::compute::{aux::Aux, simd_std::f32x8};

pub fn compute_step_tv2_simd_std(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
) {
    let alpha = f32x8::splat(alpha / (nchannel as f32).sqrt());

    for curr_row in 0..max_rounded_px_h {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            compute_step_tv2_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                alpha,
                curr_row_px_idx,
                curr_row,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_step_tv2_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32x8,
    curr_row_px_idx: u32,
    curr_row: u32,
) {
    let mut g_xxs = [f32x8!(); 3];
    let mut g_yys = [f32x8!(); 3];
    let mut g_xy_syms = [f32x8!(); 3];

    let curr_group_idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_top_edge = curr_row == 0;
    let group_at_left_edge = curr_row_px_idx == 0;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // backward difference x
        let g_yx = if group_at_left_edge {
            let a = curr_group_idx + 1;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(1..=7, aux.pixel_diff.y[a..=b]);

            let a = curr_group_idx;
            let b = curr_group_idx + 6;
            let shift_left_1px_group = f32x8!(1..=7, aux.pixel_diff.y[a..=b]);

            curr_group - shift_left_1px_group
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(&aux.pixel_diff.y[a..=b]);

            let a = curr_group_idx - 1;
            let b = curr_group_idx + 6;
            let shift_left_1px_group = f32x8!(&aux.pixel_diff.y[a..=b]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        let g_xy = if group_at_top_edge {
            f32x8!()
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(&aux.pixel_diff.x[a..=b]);

            let i = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let shift_up_1px_group = f32x8!(&aux.pixel_diff.x[i..=i + 7]);

            curr_group - shift_up_1px_group
        };

        // backward difference x
        g_xxs[c] = if group_at_left_edge {
            let a = curr_group_idx + 1;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(1..=7, aux.pixel_diff.x[a..=b]);

            let a = curr_group_idx;
            let b = curr_group_idx + 6;
            let shift_left_1px_group = f32x8!(1..=7, aux.pixel_diff.x[a..=b]);

            curr_group - shift_left_1px_group
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(&aux.pixel_diff.x[a..=b]);

            let a = curr_group_idx - 1;
            let b = curr_group_idx + 6;
            let shift_left_1px_group = f32x8!(&aux.pixel_diff.x[a..=b]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        g_yys[c] = if group_at_top_edge {
            f32x8!()
        } else {
            let a = curr_group_idx;
            let b = curr_group_idx + 7;
            let curr_group = f32x8!(&aux.pixel_diff.y[a..=b]);

            let i = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let shift_up_1px_group = f32x8!(&aux.pixel_diff.y[i..=i + 7]);

            curr_group - shift_up_1px_group
        };

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / f32x8!(2);
    }

    // gradient normalization
    let g2_norm = (0..nchannel)
        .map(|c| {
            g_xxs[c] * g_xxs[c] + f32x8!(2) * g_xy_syms[c] * g_xy_syms[c] + g_yys[c] * g_yys[c]
        })
        .fold(f32x8!(), |acc, x| acc + x)
        .sqrt();
    let mask = g2_norm.simd_ne(f32x8!());

    // compute derivatives
    for c in 0..nchannel {
        let g_xx = g_xxs[c];
        let g_yy = g_yys[c];
        let g_xy_sym = g_xy_syms[c];
        let aux = &mut auxs[c];

        '_for_current_group: {
            let a = curr_group_idx;
            let b = curr_group_idx + 7;

            let target = &mut aux.obj_gradient[a..=b];
            let result = f32x8!(target)
                + alpha * -(f32x8!(2) * g_xx + f32x8!(2) * g_xy_sym + f32x8!(2) * g_yy) / g2_norm;

            result.store_select(target, mask);
        }

        '_for_shifted_left_1px_group: {
            let update = alpha * (g_xy_sym + g_xx) / g2_norm;

            if group_at_left_edge {
                // ignore the first pixel in the group because it's out of bounds
                // [_] [0] [1] [2] [3] [4] [5] [6]
                let a = curr_group_idx;
                let b = curr_group_idx + 6;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(1..=7, target) + update;

                let mut tmp = [0.0; 8];
                result.store_select(&mut tmp, mask);
                target.copy_from_slice(&tmp[1..]);
            } else {
                let a = curr_group_idx - 1;
                let b = curr_group_idx + 6;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(target) + update;
                result.store_select(target, mask);
            }
        }

        '_for_shifted_right_1px_group: {
            let update = alpha * (g_xy_sym + g_xx) / g2_norm;

            if group_at_right_edge {
                // ignore the last pixel in the group because it's out of bounds
                // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

                let a = curr_group_idx + 1;
                let b = curr_group_idx + 7;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(shorter: target) + update;
                result.store_select(target, mask);
            } else {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 8;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(target) + update;
                result.store_select(target, mask);
            }
        }

        // for shifted up 1px group | group above the current group
        if !group_at_top_edge {
            let i = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = &mut aux.obj_gradient[i..=i + 7];
            let result = f32x8!(target) + alpha * (g_yy + g_xy_sym) / g2_norm;
            result.store_select(target, mask);
        }

        // for shifted down 1px group | group below the current group
        if !group_at_bottom_edge {
            let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = &mut aux.obj_gradient[i..=i + 7];
            let result = f32x8!(target) + alpha * (g_yy + g_xy_sym) / g2_norm;
            result.store_select(target, mask);
        }

        // for shift up right 1px group
        if !group_at_top_edge {
            let update = alpha * -g_xy_sym / g2_norm;

            if group_at_right_edge {
                // ignore the last pixel in the group because it's out of bounds
                // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

                let a = curr_group_idx + 1;
                let b = curr_group_idx + 7;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(shorter: target) + update;
                result.store_select(target, mask);
            } else {
                let a = curr_group_idx + 1;
                let b = curr_group_idx + 8;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(target) + update;
                result.store_select(target, mask);
            }
        }

        // for shift down left 1px group
        if !group_at_bottom_edge {
            let update = alpha * -g_xy_sym / g2_norm;

            if group_at_left_edge {
                // ignore the first pixel in the group because it's out of bounds
                // [_] [0] [1] [2] [3] [4] [5] [6]

                let a = curr_group_idx;
                let b = curr_group_idx + 6;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(1..=7, target) + update;

                let mut tmp = [0.0; 8];
                result.store_select(&mut tmp, mask);
                target.copy_from_slice(&tmp[1..]);
            } else {
                let a = curr_group_idx - 1;
                let b = curr_group_idx + 6;

                let target = &mut aux.obj_gradient[a..=b];
                let result = f32x8!(target) + update;
                result.store_select(target, mask);
            }
        }
    }
}
