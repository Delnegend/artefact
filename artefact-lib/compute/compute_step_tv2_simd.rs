use wide::f32x8;

use crate::compute::{aux::Aux, f32x8};

pub fn compute_step_tv2_simd(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
) {
    let alpha = alpha / (nchannel as f32).sqrt();

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
    alpha: f32,
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
            let curr_group = f32x8!(
                1..=7,
                aux.pixel_diff.y[curr_group_idx + 1..=curr_group_idx + 7]
            );
            let shift_left_1px_group =
                f32x8!(1..=7, aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        } else {
            let curr_group = f32x8!(&aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 7]);

            let shift_left_1px_group =
                f32x8!(&aux.pixel_diff.y[curr_group_idx - 1..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        let g_xy = if group_at_top_edge {
            f32x8!()
        } else {
            let curr_group = f32x8!(&aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 7]);

            let shift_up_1px_group_idx =
                ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let shift_up_1px_group =
                f32x8!(&aux.pixel_diff.x[shift_up_1px_group_idx..=shift_up_1px_group_idx + 7]);

            curr_group - shift_up_1px_group
        };

        // backward difference x
        g_xxs[c] = if group_at_left_edge {
            let curr_group = f32x8!(
                1..=7,
                aux.pixel_diff.x[curr_group_idx + 1..=curr_group_idx + 7]
            );
            let shift_left_1px_group =
                f32x8!(1..=7, aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        } else {
            let curr_group = f32x8!(&aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 7]);

            let shift_left_1px_group =
                f32x8!(&aux.pixel_diff.x[curr_group_idx - 1..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        g_yys[c] = if group_at_top_edge {
            f32x8!()
        } else {
            let curr_group = f32x8!(&aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 7]);

            let shift_up_1px_group_idx =
                ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let shift_up_1px_group =
                f32x8!(&aux.pixel_diff.y[shift_up_1px_group_idx..=shift_up_1px_group_idx + 7]);

            curr_group - shift_up_1px_group
        };

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / 2.0;
    }

    // gradient normalization
    let g2_norm = (0..nchannel)
        .map(|c| g_xxs[c] * g_xxs[c] + 2.0 * g_xy_syms[c] * g_xy_syms[c] + g_yys[c] * g_yys[c])
        .fold(f32x8!(), |acc, x| acc + x)
        .sqrt();

    // compute derivatives
    for c in 0..nchannel {
        let g_xx = g_xxs[c];
        let g_yy = g_yys[c];
        let g_xy_sym = g_xy_syms[c];
        let aux = &mut auxs[c];

        '_for_current_group: {
            let target = aux.obj_gradient[curr_group_idx..=curr_group_idx + 7].as_mut();

            let original = f32x8!(&target[..]);
            let update = f32x8!(div: alpha * -(2.0 * g_xx + 2.0 * g_xy_sym + 2.0 * g_yy), g2_norm);

            target.copy_from_slice((original + update).as_array_ref());
        }

        '_for_shifted_left_1px_group: {
            let update = f32x8!(div: alpha * (g_xy_sym + g_xx), g2_norm);

            if group_at_left_edge {
                // ignore the first pixel in the group because it's out of bounds
                // [_] [0] [1] [2] [3] [4] [5] [6]

                let target = &mut aux.obj_gradient[curr_group_idx..=curr_group_idx + 6];
                let original = f32x8!(1..=7, target);

                target.copy_from_slice(&(original + update).as_array_ref()[1..]);
            } else {
                let target = aux.obj_gradient[curr_group_idx - 1..=curr_group_idx + 6].as_mut();
                let original = f32x8!(&target[..]);

                target.copy_from_slice((original + update).as_array_ref());
            }
        }

        '_for_shifted_right_1px_group: {
            let update = f32x8!(div: alpha * (g_xy_sym + g_xx), g2_norm);

            if group_at_right_edge {
                // ignore the last pixel in the group because it's out of bounds
                // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

                let target = &mut aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7];
                let original = f32x8!(..=6, target);

                target.copy_from_slice(&(original + update).as_array_ref()[..=6]);
            } else {
                let target = aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 8].as_mut();
                let original = f32x8!(&target[..]);

                target.copy_from_slice((original + update).as_array_ref());
            }
        }

        // for shifted up 1px group | group above the current group
        if !group_at_top_edge {
            let start = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = aux.obj_gradient[start..=start + 7].as_mut();
            let original = f32x8!(&target[..]);
            let update = f32x8!(div: alpha * (g_yy + g_xy_sym), g2_norm);

            target.copy_from_slice((original + update).as_array_ref());
        }

        // for shifted down 1px group | group below the current group
        if !group_at_bottom_edge {
            let start = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = aux.obj_gradient[start..=start + 7].as_mut();
            let original = f32x8!(&target[..]);
            let update = f32x8!(div: alpha * (g_yy + g_xy_sym), g2_norm);

            target.copy_from_slice((original + update).as_array_ref());
        }

        // for shift up right 1px group
        if !group_at_top_edge {
            let update = f32x8!(div: alpha * -g_xy_sym, g2_norm);

            if group_at_right_edge {
                // ignore the last pixel in the group because it's out of bounds
                // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

                let target = &mut aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7];
                let original = f32x8!(..=6, target);

                target.copy_from_slice(&(original + update).as_array_ref()[..=6]);
            } else {
                let target = aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 8].as_mut();
                let original = f32x8!(&target[..]);

                target.copy_from_slice((original + update).as_array_ref());
            }
        }

        // for shift down left 1px group
        if !group_at_bottom_edge {
            let update = f32x8!(div: alpha * -g_xy_sym, g2_norm);

            if group_at_left_edge {
                // ignore the first pixel in the group because it's out of bounds
                // [_] [0] [1] [2] [3] [4] [5] [6]

                let target = &mut aux.obj_gradient[curr_group_idx..=curr_group_idx + 6];
                let original = f32x8!(1..=7, target);

                target.copy_from_slice(&(original + update).as_array_ref()[1..]);
            } else {
                let target = aux.obj_gradient[curr_group_idx - 1..=curr_group_idx + 6].as_mut();
                let original = f32x8!(&target[..]);

                target.copy_from_slice((original + update).as_array_ref());
            }
        }
    }
}
