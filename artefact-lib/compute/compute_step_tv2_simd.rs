use wide::{f32x8, f64x4};

use crate::compute::aux::Aux;

#[allow(unused)]
pub fn compute_step_tv2_simd(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
) -> f64 {
    let mut tv2 = 0.0_f64;

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
                &mut tv2,
            );
        }
    }

    tv2
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
    tv2: &mut f64,
) {
    let mut g_xxs = [f32x8::splat(0.0); 3];
    let mut g_yys = [f32x8::splat(0.0); 3];
    let mut g_xy_syms = [f32x8::splat(0.0); 3];

    let curr_group_idx = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_top_edge = curr_row == 0;
    let group_at_left_edge = curr_row_px_idx == 0;
    let group_at_bottom_edge = curr_row == max_rounded_px_h - 1;
    let group_at_right_edge = curr_row_px_idx + 8 >= max_rounded_px_w;

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // backward difference x
        let g_yx = if group_at_left_edge {
            let curr_group = f32x8::from({
                let mut tmp = [0.0; 8];
                tmp[1..=7]
                    .copy_from_slice(&aux.pixel_diff.y[curr_group_idx + 1..=curr_group_idx + 7]);
                tmp
            });
            let shift_left_1px_group = f32x8::from({
                let mut tmp = [0.0; 8];
                tmp[1..=7].copy_from_slice(&aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 6]);
                tmp
            });

            curr_group - shift_left_1px_group
        } else {
            let curr_group = f32x8::from(&aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 7]);

            let shift_left_1px_group =
                f32x8::from(&aux.pixel_diff.y[curr_group_idx - 1..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        let g_xy = if group_at_top_edge {
            f32x8::splat(0.0)
        } else {
            let curr_group = f32x8::from(&aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 7]);

            let shift_up_1px_group_idx =
                ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let shift_up_1px_group =
                f32x8::from(&aux.pixel_diff.x[shift_up_1px_group_idx..=shift_up_1px_group_idx + 7]);

            curr_group - shift_up_1px_group
        };

        // backward difference x
        g_xxs[c] = if group_at_left_edge {
            let curr_group = f32x8::from({
                let mut tmp = [0.0; 8];
                tmp[1..=7]
                    .copy_from_slice(&aux.pixel_diff.x[curr_group_idx + 1..=curr_group_idx + 7]);
                tmp
            });
            let shift_left_1px_group = f32x8::from({
                let mut tmp = [0.0; 8];
                tmp[1..=7].copy_from_slice(&aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 6]);
                tmp
            });

            curr_group - shift_left_1px_group
        } else {
            let curr_group = f32x8::from(&aux.pixel_diff.x[curr_group_idx..=curr_group_idx + 7]);

            let shift_left_1px_group =
                f32x8::from(&aux.pixel_diff.x[curr_group_idx - 1..=curr_group_idx + 6]);

            curr_group - shift_left_1px_group
        };

        // backward difference y
        g_yys[c] = if group_at_top_edge {
            f32x8::splat(0.0)
        } else {
            let curr_group = f32x8::from(&aux.pixel_diff.y[curr_group_idx..=curr_group_idx + 7]);

            let shift_up_1px_group_idx =
                ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;

            let shift_up_1px_group =
                f32x8::from(&aux.pixel_diff.y[shift_up_1px_group_idx..=shift_up_1px_group_idx + 7]);

            curr_group - shift_up_1px_group
        };

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / 2.0;
    }

    let alpha_single = (alpha) * 1.0 / (nchannel as f32).sqrt();
    let alpha_f32 = f32x8::splat(alpha_single);
    let alpha_f64 = f64x4::splat(alpha_single as f64);

    // norm
    let g2_norm_f32 = {
        let mut f32_ver = f32x8::splat(0.0);
        let mut f64_ver = (f64x4::splat(0.0), f64x4::splat(0.0));

        for c in 0..nchannel {
            f32_ver +=
                g_xxs[c] * g_xxs[c] + 2.0 * g_xy_syms[c] * g_xy_syms[c] + g_yys[c] * g_yys[c];

            let g_xxs = {
                let tmp = g_xxs[c].as_array_ref().map(|x| x as f64);
                (f64x4::from(&tmp[..4]), f64x4::from(&tmp[4..]))
            };
            let g_xy_syms = {
                let tmp = g_xy_syms[c].as_array_ref().map(|x| x as f64);
                (f64x4::from(&tmp[..4]), f64x4::from(&tmp[4..]))
            };
            let g_yys = {
                let tmp = g_yys[c].as_array_ref().map(|x| x as f64);
                (f64x4::from(&tmp[..4]), f64x4::from(&tmp[4..]))
            };

            f64_ver.0 +=
                (g_xxs.0 * g_xxs.0) + (2.0 * g_xy_syms.0 * g_xy_syms.0) + (g_yys.0 * g_yys.0);
            f64_ver.1 +=
                (g_xxs.1 * g_xxs.1) + (2.0 * g_xy_syms.1 * g_xy_syms.1) + (g_yys.1 * g_yys.1);
        }

        *tv2 += (alpha_f64 * (f64_ver.0.sqrt() + f64_ver.1.sqrt())).reduce_add();

        f32_ver.sqrt()
    };

    // compute derivatives
    for c in 0..nchannel {
        let g_xx = g_xxs[c];
        let g_yy = g_yys[c];
        let g_xy_sym = g_xy_syms[c];
        let aux = &mut auxs[c];

        '_for_current_group: {
            let target = aux.obj_gradient[curr_group_idx..=curr_group_idx + 7].as_mut();

            let original = f32x8::from(&target[..]);
            let update = {
                let dividend = -(2.0 * g_xx + 2.0 * g_xy_sym + 2.0 * g_yy);
                match g2_norm_f32.as_array_ref() {
                    g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                        g2_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g_norm)| match g_norm {
                                0.0 => 0.0,
                                _ => alpha_single * dividend.as_array_ref()[i] / g_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * dividend / g2_norm_f32,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_left_1px_group: {
            // the value to be += to the target stays the same
            let update = {
                let dividend = g_xy_sym + g_xx;
                match g2_norm_f32.as_array_ref() {
                    g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                        g2_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g2_norm)| match g2_norm {
                                0.0 => 0.0,
                                _ => alpha_single * dividend.as_array_ref()[i] / g2_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * dividend / g2_norm_f32,
                }
            };

            // ignore the first pixel in the group because it's out of bounds
            // [_] [0] [1] [2] [3] [4] [5] [6]
            if group_at_left_edge {
                let original = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[1..=7]
                        .copy_from_slice(&aux.obj_gradient[curr_group_idx..=curr_group_idx + 6]);
                    tmp
                });

                aux.obj_gradient[curr_group_idx..=curr_group_idx + 6]
                    .copy_from_slice(&(original + update).as_array_ref()[1..]);

                break 'for_shifted_left_1px_group;
            }

            let target = aux.obj_gradient[curr_group_idx - 1..=curr_group_idx + 6].as_mut();

            let original = f32x8::from(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_right_1px_group: {
            let update = {
                let dividend = g_xy_sym + g_xx;
                match g2_norm_f32.as_array_ref() {
                    g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                        g2_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g2_norm)| match g2_norm {
                                0.0 => 0.0,
                                _ => alpha_single * dividend.as_array_ref()[i] / g2_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * dividend / g2_norm_f32,
                }
            };

            // ignore the last pixel in the group because it's out of bounds
            // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)
            if group_at_right_edge {
                let original = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[..=6].copy_from_slice(
                        &aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7],
                    );
                    tmp
                });

                aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7]
                    .copy_from_slice(&(original + update).as_array_ref()[..=6]);

                break 'for_shifted_right_1px_group;
            }

            let target = aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 8].as_mut();

            let original = f32x8::from(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }

        // for shifted up 1px group | group above the current group
        if !group_at_top_edge {
            let start = ((curr_row - 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let target = aux.obj_gradient[start..=start + 7].as_mut();

            let original = f32x8::from(&target[..]);
            let update = {
                let dividend = g_yys[c] + g_xy_syms[c];
                match g2_norm_f32.as_array_ref() {
                    g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                        g2_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g2_norm)| match g2_norm {
                                0.0 => 0.0,
                                _ => alpha_single * dividend.as_array_ref()[i] / g2_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * dividend / g2_norm_f32,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }

        // for shifted down 1px group | group below the current group
        if !group_at_bottom_edge {
            let start = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let target = aux.obj_gradient[start..=start + 7].as_mut();

            let original = f32x8::from(&target[..]);
            let update = {
                let dividend = g_yy + g_xy_sym;
                match g2_norm_f32.as_array_ref() {
                    g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                        g2_norm
                            .iter()
                            .enumerate()
                            .map(|(i, g2_norm)| match g2_norm {
                                0.0 => 0.0,
                                _ => alpha_single * dividend.as_array_ref()[i] / g2_norm,
                            })
                            .collect::<Vec<f32>>()
                            .as_slice(),
                    ),
                    _ => alpha_f32 * dividend / g2_norm_f32,
                }
            };

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_up_right_1px_group: {
            // we need to shift up but we can't
            if group_at_top_edge {
                break 'for_shifted_up_right_1px_group;
            }

            let update = match g2_norm_f32.as_array_ref() {
                g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                    g2_norm
                        .iter()
                        .enumerate()
                        .map(|(i, g2_norm)| match g2_norm {
                            0.0 => 0.0,
                            _ => alpha_single * -g_xy_sym.as_array_ref()[i] / g2_norm,
                        })
                        .collect::<Vec<f32>>()
                        .as_slice(),
                ),
                _ => alpha_f32 * -g_xy_sym / g2_norm_f32,
            };

            // ignore the last pixel in the group because it's out of bounds
            // [1] [2] [3] [4] [5] [6] [7] [_] (i.e the image width is 8px)
            if group_at_right_edge {
                let original = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[..=6].copy_from_slice(
                        &aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7],
                    );
                    tmp
                });

                aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 7]
                    .copy_from_slice(&(original + update).as_array_ref()[..=6]);

                break 'for_shifted_up_right_1px_group;
            }

            let target = aux.obj_gradient[curr_group_idx + 1..=curr_group_idx + 8].as_mut();

            let original = f32x8::from(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }

        'for_shifted_down_left_1px_group: {
            // we need to shift down but we can't
            if group_at_bottom_edge {
                break 'for_shifted_down_left_1px_group;
            }

            let update = match g2_norm_f32.as_array_ref() {
                g2_norm if g2_norm.contains(&0.0) => f32x8::from(
                    g2_norm
                        .iter()
                        .enumerate()
                        .map(|(i, g2_norm)| match g2_norm {
                            0.0 => 0.0,
                            _ => alpha_single * -g_xy_sym.as_array_ref()[i] / g2_norm,
                        })
                        .collect::<Vec<f32>>()
                        .as_slice(),
                ),
                _ => alpha_f32 * -g_xy_sym / g2_norm_f32,
            };

            // ignore the first pixel in the group because it's out of bounds
            // [_] [0] [1] [2] [3] [4] [5] [6]
            if group_at_left_edge {
                let original = f32x8::from({
                    let mut tmp = [0.0; 8];
                    tmp[1..=7]
                        .copy_from_slice(&aux.obj_gradient[curr_group_idx..=curr_group_idx + 6]);
                    tmp
                });

                aux.obj_gradient[curr_group_idx..=curr_group_idx + 6]
                    .copy_from_slice(&(original + update).as_array_ref()[1..]);

                break 'for_shifted_down_left_1px_group;
            }

            let target = aux.obj_gradient[curr_group_idx - 1..=curr_group_idx + 6].as_mut();

            let original = f32x8::from(&target[..]);

            target.copy_from_slice((original + update).as_array_ref());
        }
    }
}
