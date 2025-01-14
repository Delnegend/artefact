use wide::f32x8;

use crate::compute::{aux::Aux, simd::f32x8};

pub fn compute_step_tv_simd(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let alpha = 1.0 / (nchannel as f32).sqrt();

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
    alpha: f32,
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
            let curr_group = f32x8!(..=6, &aux.fdata[a..=b]);

            let a = px_idx_start_of_group + 1;
            let b = px_idx_start_of_group + 7;
            let shift_right_1px_group = f32x8!(..=6, &aux.fdata[a..=b]);

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

    // compute derivatives
    for c in 0..nchannel {
        let aux = &mut auxs[c];

        '_for_current_group: {
            let target = &mut aux.obj_gradient[px_idx_start_of_group..=px_idx_start_of_group + 7];
            let original = f32x8!(&target[..]);
            let update = f32x8!(div: alpha * -(g_xs[c] + g_ys[c]), g_norm);

            target.copy_from_slice((original + update).as_array_ref());
        }

        '_for_shifted_right_1px_group: {
            let update = f32x8!(div: alpha * g_xs[c], g_norm);

            if group_at_right_edge {
                // ignore the last pixel in the group because it's out of bounds
                // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

                let a = px_idx_start_of_group + 1;
                let b = px_idx_start_of_group + 7;
                let target = &mut aux.obj_gradient[a..=b];
                let original = f32x8!(..=6, &target[..]);

                target.copy_from_slice(&(original + update).as_array_ref()[..=6]);
            } else {
                let a = px_idx_start_of_group + 1;
                let b = px_idx_start_of_group + 8;
                let target = &mut aux.obj_gradient[a..=b];
                let original = f32x8!(&target[..]);

                target.copy_from_slice((original + update).as_array_ref());
            }
        }

        // for shifted_down_1px_group aka group below the current group
        if !group_at_bottom_edge {
            let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let target = aux.obj_gradient[i..=i + 7].as_mut();
            let original = f32x8!(&target[..]);
            let update = f32x8!(div: alpha * g_ys[c], g_norm);

            target.copy_from_slice((original + update).as_array_ref());
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        let a = px_idx_start_of_group;
        let b = px_idx_start_of_group + 7;

        auxs[c].pixel_diff.x[a..=b].copy_from_slice(g_xs[c].as_array_ref());
        if !group_at_bottom_edge {
            auxs[c].pixel_diff.y[a..=b].copy_from_slice(g_ys[c].as_array_ref());
        }
    }
}
