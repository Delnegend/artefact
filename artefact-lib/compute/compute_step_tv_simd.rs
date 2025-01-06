use wide::f32x8;

use crate::compute::aux::Aux;

pub fn compute_step_tv_simd(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    for curr_row in 0..max_rounded_px_h {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            compute_step_tv_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                curr_row_px_idx,
                curr_row,
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
) {
    // a "group" = 8 consecutive pixels horizontally

    let curr_group_idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;

    let mut g_xs = [f32x8::splat(0.0); 3];
    let mut g_ys = [f32x8::splat(0.0); 3];

    // compute forward differences
    for c in 0..nchannel {
        let aux = &auxs[c];

        // forward difference x
        g_xs[c] = match group_at_right_edge {
            true => {
                // only handle 7 consecutive pixels because the last one is at the
                // edge, and there's no more pixel to the right for us to calculate
                // the difference with

                let curr_px_group = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[0..=6].copy_from_slice(&aux.fdata[curr_group_idx..=curr_group_idx + 6]);
                    tmp
                });

                let shift_right_1px_group = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[0..=6].copy_from_slice(&aux.fdata[curr_group_idx + 1..=curr_group_idx + 7]);
                    tmp
                });

                shift_right_1px_group - curr_px_group
            }
            false => {
                // 8 pixels

                let curr_group = f32x8::from(&aux.fdata[curr_group_idx..=curr_group_idx + 7]);

                let shift_right_1px_group =
                    f32x8::from(&aux.fdata[curr_group_idx + 1..=curr_group_idx + 8]);

                shift_right_1px_group - curr_group
            }
        };

        // forward difference y
        if !group_at_bottom_edge {
            let curr_group = f32x8::from(&aux.fdata[curr_group_idx..=curr_group_idx + 7]);

            let shift_down_1px_group_idx =
                ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let shift_down_1px_group =
                f32x8::from(&aux.fdata[shift_down_1px_group_idx..=shift_down_1px_group_idx + 7]);

            g_ys[c] = shift_down_1px_group - curr_group
        }
    }

    let alpha = 1.0 / (nchannel as f32).sqrt();
    let alpha_f32 = f32x8::splat(alpha);

    // compute gradient normalization
    // f64 for high-precision for tv; f32 for low-precision for derivatives aka obj_gradient
    let g_norm = {
        let mut f32_ver = f32x8::splat(0.0);
        for c in 0..nchannel {
            f32_ver += g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c];
        }
        f32_ver.sqrt()
    };

    // compute derivatives
    for c in 0..nchannel {
        let aux = &mut auxs[c];

        '_for_current_group: {
            let target = &mut aux.obj_gradient[curr_group_idx..=curr_group_idx + 7];

            let original = f32x8::from(&target[..]);

            let update = {
                let dividend = match group_at_bottom_edge {
                    true => alpha_f32 * -g_xs[c],
                    false => alpha_f32 * -(g_xs[c] + g_ys[c]),
                };

                match g_norm.as_array_ref() {
                    g_norm if g_norm.contains(&0.0) => f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend.as_array_ref()[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => dividend / g_norm,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_right_1px_group: {
            let update = {
                let dividend = alpha_f32 * g_xs[c];

                match g_norm.as_array_ref() {
                    g_norm if g_norm.contains(&0.0) => f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend.as_array_ref()[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => dividend / g_norm,
                }
            };

            // ignore the last pixel in the group because it's out of bounds
            // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)
            if group_at_right_edge {
                let original = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[0..=6].copy_from_slice(
                        &aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7],
                    );
                    tmp
                });

                aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7]
                    .copy_from_slice(&(original + update).as_array_ref()[..=6]);

                break 'for_shifted_right_1px_group;
            }

            let target = &mut aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 8];

            let original = f32x8::from(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }

        // for shifted_down_1px_group aka group below the current group
        if !group_at_bottom_edge {
            let start = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let target = aux.obj_gradient[start..=start + 7].as_mut();

            let original = f32x8::from(&target[..]);
            let update = {
                let dividend = match group_at_bottom_edge {
                    true => alpha_f32,
                    false => alpha_f32 * g_ys[c],
                };

                match g_norm.as_array_ref() {
                    g_norm if g_norm.contains(&0.0) => f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend.as_array_ref()[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => dividend / g_norm,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        auxs[c].pixel_diff.x[curr_group_idx..=curr_group_idx + 7]
            .copy_from_slice(g_xs[c].as_array_ref());

        if !group_at_bottom_edge {
            auxs[c].pixel_diff.y[curr_group_idx..=curr_group_idx + 7]
                .copy_from_slice(g_ys[c].as_array_ref());
        }
    }
}
