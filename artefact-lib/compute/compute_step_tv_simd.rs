use crate::compute::aux::Aux;
use wide::{f32x8, f64x4};

pub fn compute_step_tv_simd(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) -> f64 {
    // TODO: replace with f64x8 when wide supports it
    // let mut tv = [f64x4::splat(0.0); 2];
    let mut tv = 0.0_f64;

    let alpha = 1.0 / (nchannel as f32).sqrt();
    let alpha_f32 = f32x8::splat(alpha);
    let alpha_f64 = f64x4::splat(alpha as f64);

    for curr_row in 0..max_rounded_px_h {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            compute_step_tv_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                curr_row_px_idx,
                curr_row,
                &mut tv,
                // hoisting the constants out of the loop
                alpha_f32,
                alpha_f64,
            );
        }
    }

    tv
}

fn compute_step_tv_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_px_idx: u32,
    curr_row: u32,
    tv: &mut f64,
    // hoisted constants
    alpha_f32: f32x8,
    alpha_f64: f64x4,
) {
    // a "group" = 8 consecutive pixels horizontally

    let curr_group_idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;

    let mut g_xs = [f32x8::splat(0.0); 3];
    let mut g_ys = [f32x8::splat(0.0); 3];

    // compute forward differences
    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // forward difference x
        g_xs[c] = if group_at_right_edge {
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
        } else {
            // 8 pixels

            let curr_group = f32x8::from(&aux.fdata[curr_group_idx..=curr_group_idx + 7]);

            let shift_right_1px_group =
                f32x8::from(&aux.fdata[curr_group_idx + 1..=curr_group_idx + 8]);

            shift_right_1px_group - curr_group
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

    // compute gradient normalization
    let (g_norm_f32, g_norm_f64) = {
        let mut f32_ver = f32x8::splat(0.0);
        let mut f64_ver = (f64x4::splat(0.0), f64x4::splat(0.0));

        for c in 0..nchannel {
            // only if there were f64x8 in wide...

            // from 1 f64x8 to 2 f64x4
            let g_xs_parts = {
                let tmp = g_xs[c].as_array_ref().map(|x| x as f64);
                (
                    f64x4::from([tmp[0], tmp[1], tmp[2], tmp[3]]),
                    f64x4::from([tmp[4], tmp[5], tmp[6], tmp[7]]),
                )
            };
            f64_ver.0 += g_xs_parts.0 * g_xs_parts.0;
            f64_ver.1 += g_xs_parts.1 * g_xs_parts.1;

            // from 1 f64x8 to 2 f64x4
            if !group_at_bottom_edge {
                let g_ys_parts = {
                    let tmp = g_ys[c].as_array_ref().map(|x| x as f64);
                    (
                        f64x4::from([tmp[0], tmp[1], tmp[2], tmp[3]]),
                        f64x4::from([tmp[4], tmp[5], tmp[6], tmp[7]]),
                    )
                };

                f64_ver.0 += g_ys_parts.0 * g_ys_parts.0;
                f64_ver.1 += g_ys_parts.1 * g_ys_parts.1;
            }

            if group_at_bottom_edge {
                f32_ver += g_xs[c] * g_xs[c];
            } else {
                f32_ver += g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c];
            }
        }

        f64_ver.0 = f64_ver.0.sqrt();
        f64_ver.1 = f64_ver.1.sqrt();
        f32_ver = f32_ver.sqrt();

        (f32_ver, f64_ver)
    };

    // // f64 for high-precision for tv; f32 for low-precision for derivatives aka obj_gradient
    // let alpha = 1.0 / (nchannel as f32).sqrt();
    // let alpha_f32 = f32x8::splat(alpha);
    // let alpha_f64 = f64x4::splat(alpha as f64);
    // hoisted outside

    *tv += (alpha_f64 * g_norm_f64.0).reduce_add();
    *tv += (alpha_f64 * g_norm_f64.1).reduce_add();

    // compute derivatives
    for c in 0..nchannel {
        let aux = &mut auxs[c];

        '_for_current_group: {
            let target = &mut aux.obj_gradient[curr_group_idx..=curr_group_idx + 7];

            let original = f32x8::from(&target[..]);

            let update = {
                let dividend = alpha_f32 * -(g_xs[c] + g_ys[c]);
                let dividend_ref = dividend.as_array_ref();

                match g_norm_f32.as_array_ref() {
                    g_norm if g_norm.contains(&0.0) => f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend_ref[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * -(g_xs[c] + g_ys[c]) / g_norm_f32,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_right_1px_group: {
            let update = match g_norm_f32.as_array_ref() {
                g_norm if g_norm.contains(&0.0) => {
                    let dividend = alpha_f32 * g_xs[c];
                    let dividend_ref = dividend.as_array_ref();

                    f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend_ref[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    )
                }
                _ => alpha_f32 * g_xs[c] / g_norm_f32,
            };

            // ignore the last pixel in the group because it's out of bounds
            // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)
            if group_at_right_edge {
                let original = f32x8::from([
                    aux.obj_gradient[curr_group_idx + 1],
                    aux.obj_gradient[curr_group_idx + 2],
                    aux.obj_gradient[curr_group_idx + 3],
                    aux.obj_gradient[curr_group_idx + 4],
                    aux.obj_gradient[curr_group_idx + 5],
                    aux.obj_gradient[curr_group_idx + 6],
                    aux.obj_gradient[curr_group_idx + 7],
                    0.0,
                ]);

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
            let update = match g_norm_f32.as_array_ref() {
                g_norm if g_norm.contains(&0.0) => {
                    let dividend = alpha_f32 * g_ys[c];
                    let dividend_ref = dividend.as_array_ref();

                    f32x8::from(
                        g_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match *g_norm {
                                0.0 => 0.0,
                                _ => dividend_ref[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    )
                }
                _ => alpha_f32 * g_ys[c] / g_norm_f32,
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
