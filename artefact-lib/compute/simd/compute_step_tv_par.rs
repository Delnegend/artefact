use rayon::prelude::*;
use wide::f32x8;

use crate::compute::{aux::Aux, simd::f32x8};

/// A slower version (for some reason) of [`compute_step_tv_simd`] with
/// [`rayon`] parallelization.
///
/// [`compute_step_tv_simd`]: crate::compute::simd::compute_step_tv::compute_step_tv_simd
#[allow(unused)]
pub fn compute_step_tv_simd_par(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let alpha = 1.0 / (nchannel as f32).sqrt();
    let max_rounded_px_count = (max_rounded_px_w * max_rounded_px_h) as usize;
    let group_count = max_rounded_px_count / 8;

    let chans_forward_diffs = (0..nchannel)
        .into_par_iter()
        .map(|c| {
            let mut chan_g_xs = vec![f32x8!(); group_count];
            let mut chan_g_ys = vec![f32x8!(); group_count];

            for curr_row in 0..max_rounded_px_h {
                for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
                    compute_forward_differents(
                        max_rounded_px_w,
                        max_rounded_px_h,
                        &auxs[c],
                        curr_row_px_idx,
                        curr_row,
                        &mut chan_g_xs,
                        &mut chan_g_ys,
                    );
                }
            }

            (chan_g_xs, chan_g_ys)
        })
        .collect::<Vec<_>>();

    let mut g_norm = vec![f32x8!(); group_count];
    for group_idx in 0..group_count {
        for (chan_g_xs, chan_g_ys) in chans_forward_diffs.iter() {
            let g_xs = chan_g_xs[group_idx];
            let g_ys = chan_g_ys[group_idx];
            g_norm[group_idx] += (g_xs * g_xs + g_ys * g_ys).sqrt();
        }
    }

    auxs.par_iter_mut()
        .zip(chans_forward_diffs)
        .for_each(|(aux, (g_xs, g_ys))| {
            for curr_row in 0..max_rounded_px_h {
                for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
                    let px_idx_start_of_group =
                        (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
                    let curr_group_idx =
                        (curr_row * max_rounded_px_w / 8 + curr_row_px_idx / 8) as usize;

                    compute_derivatives(
                        curr_row,
                        curr_row_px_idx,
                        max_rounded_px_w,
                        max_rounded_px_h,
                        aux,
                        px_idx_start_of_group,
                        g_xs[curr_group_idx],
                        g_ys[curr_group_idx],
                        alpha,
                        &g_norm[curr_group_idx],
                    );
                }
            }
        });
}

fn compute_forward_differents(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &Aux,
    curr_row_px_idx: u32,
    curr_row: u32,
    chan_g_xs: &mut [f32x8],
    chan_g_ys: &mut [f32x8],
) {
    let curr_px_idx_start_of_group = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;
    let group_idx = (curr_row * max_rounded_px_w / 8 + curr_row_px_idx / 8) as usize;

    chan_g_xs[group_idx] = if group_at_right_edge {
        // groups_at_right_edge.insert(curr_px_idx_start_of_group);

        // only handle 7 consecutive pixels because the last one is at the
        // edge, and there's no more pixel to the right for us to calculate
        // the difference with

        let curr_group = f32x8!(
            ..=6,
            &aux.fdata[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 6]
        );
        let shift_right_1px_group = f32x8!(
            ..=6,
            &aux.fdata[curr_px_idx_start_of_group + 1..=curr_px_idx_start_of_group + 7]
        );

        shift_right_1px_group - curr_group
    } else {
        // 8 pixels

        let curr_group =
            f32x8!(&aux.fdata[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 7]);
        let shift_right_1px_group =
            f32x8!(&aux.fdata[curr_px_idx_start_of_group + 1..=curr_px_idx_start_of_group + 8]);

        shift_right_1px_group - curr_group
    };

    // forward difference y
    if !group_at_bottom_edge {
        let curr_group =
            f32x8!(&aux.fdata[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 7]);

        let shift_down_1px_group_idx =
            ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
        let shift_down_1px_group =
            f32x8!(&aux.fdata[shift_down_1px_group_idx..=shift_down_1px_group_idx + 7]);

        chan_g_ys[group_idx] = shift_down_1px_group - curr_group;
    };
}

#[allow(clippy::too_many_arguments)]
fn compute_derivatives(
    curr_row: u32,
    curr_row_px_idx: u32,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    curr_px_idx_start_of_group: usize,
    g_xs: f32x8,
    g_ys: f32x8,
    alpha: f32,
    g_norm: &f32x8,
) {
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;

    '_for_current_group: {
        let target =
            &mut aux.obj_gradient[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 7];
        let original = f32x8!(&target[..]);
        let update = f32x8!(div: alpha * -(g_xs + g_ys), g_norm);

        target.copy_from_slice((original + update).as_array_ref());
    }

    '_for_shifted_right_1px_group: {
        let update = f32x8!(div: alpha * g_xs, g_norm);

        if group_at_right_edge {
            // ignore the last pixel in the group because it's out of bounds
            // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)

            let target = &mut aux.obj_gradient
                [curr_px_idx_start_of_group + 1..=curr_px_idx_start_of_group + 7];
            let original = f32x8!(..=6, &target[..]);

            target.copy_from_slice(&(original + update).as_array_ref()[..=6]);
        } else {
            let target = &mut aux.obj_gradient
                [curr_px_idx_start_of_group + 1..=curr_px_idx_start_of_group + 8];
            let original = f32x8!(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }
    }

    // for shifted_down_1px_group aka group below the current group
    if !group_at_bottom_edge {
        let start = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

        let target = aux.obj_gradient[start..=start + 7].as_mut();
        let original = f32x8!(&target[..]);
        let update = f32x8!(div: alpha * g_ys, g_norm);

        target.copy_from_slice((original + update).as_array_ref());
    }

    // store for use in tv2
    aux.pixel_diff.x[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 7]
        .copy_from_slice(g_xs.as_array_ref());

    if !group_at_bottom_edge {
        aux.pixel_diff.y[curr_px_idx_start_of_group..=curr_px_idx_start_of_group + 7]
            .copy_from_slice(g_ys.as_array_ref());
    }
}
