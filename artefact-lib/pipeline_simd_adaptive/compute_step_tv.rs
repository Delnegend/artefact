use std::{
    ops::Div,
    simd::{cmp::SimdPartialEq, f32x16, f32x32, f32x64, f32x8, StdFloat},
};

use paste::paste;

use super::adaptive_width::AdaptiveWidth;
use crate::utils::{
    aux::Aux,
    traits::{AddSlice, FromSlice, WriteTo},
};

pub fn compute_step_tv(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    adaptive_widths: &[AdaptiveWidth],
) {
    for curr_row in 0..max_rounded_px_h {
        macro_rules! gen_caller {
            ($width:literal, $curr_row_px_idx:ident) => {
                paste! {
                    [<compute_step_tv_inner_ $width>](
                        max_rounded_px_w,
                        max_rounded_px_h,
                        nchannel,
                        auxs,
                        *$curr_row_px_idx,
                        curr_row,
                    )
                }
            };
        }

        for adaptive_width in adaptive_widths {
            match adaptive_width {
                AdaptiveWidth::X8(x) => gen_caller!(8, x),
                AdaptiveWidth::X16(x) => gen_caller!(16, x),
                AdaptiveWidth::X32(x) => gen_caller!(32, x),
                AdaptiveWidth::X64(x) => gen_caller!(64, x),
            }
        }
    }
}

macro_rules! gen_func {
    (width: $width:literal, pad: $pad:tt) => {
        paste! {
            fn [<compute_step_tv_inner_ $width>](
                max_rounded_px_w: u32,
                max_rounded_px_h: u32,
                nchannel: usize,
                auxs: &mut [Aux],
                curr_row_px_idx: u32,
                curr_row: u32,
            ) {
                let px_idx_start_of_group = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
                let group_at_right_edge = curr_row_px_idx + 8 + $pad == max_rounded_px_w;
                let group_at_bottom_edge = curr_row + 1 == max_rounded_px_h;

                let mut g_xs = [[<f32x $width>]::splat(0.0); 3];
                let mut g_ys = [[<f32x $width>]::splat(0.0); 3];

                // compute forward differences
                for c in 0..nchannel {
                    let aux = &auxs[c];

                    // forward difference x
                    g_xs[c] = if group_at_right_edge {
                        let a = px_idx_start_of_group;
                        let b = px_idx_start_of_group + 6 + $pad;
                        let curr_group = [<f32x $width>]::from_short_slc(&aux.fdata[a..=b]);

                        let a = px_idx_start_of_group + 1;
                        let b = px_idx_start_of_group + 7 + $pad;
                        let shift_right_1px_group = [<f32x $width>]::from_short_slc(&aux.fdata[a..=b]);

                        shift_right_1px_group - curr_group
                    } else {
                        let a = px_idx_start_of_group;
                        let b = px_idx_start_of_group + 7 + $pad;
                        let curr_group = [<f32x $width>]::from_slc(&aux.fdata[a..=b]);

                        let a = px_idx_start_of_group + 1;
                        let b = px_idx_start_of_group + 8 + $pad;
                        let shift_right_1px_group = [<f32x $width>]::from_slc(&aux.fdata[a..=b]);

                        shift_right_1px_group - curr_group
                    };

                    // forward difference y
                    if !group_at_bottom_edge {
                        let a = px_idx_start_of_group;
                        let b = px_idx_start_of_group + 7 + $pad;
                        let curr_group = [<f32x $width>]::from_slc(&aux.fdata[a..=b]);

                        let a = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
                        let b = a + 7 + $pad;
                        let shift_down_1px_group = [<f32x $width>]::from_slc(&aux.fdata[a..=b]);

                        g_ys[c] = shift_down_1px_group - curr_group;
                    }
                }

                // compute gradient normalization
                let alpha = [<f32x $width>]::splat(1.0 / (nchannel as f32).sqrt());
                let g_norm = (0..nchannel)
                    .map(|c| g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c])
                    .fold([<f32x $width>]::splat(0.0), |acc, x| acc + x)
                    .sqrt();
                let mask = g_norm.simd_ne([<f32x $width>]::splat(0.0));

                for c in 0..nchannel {
                    // ===== compute derivatives =====
                    let aux = &mut auxs[c];

                    '_for_current_group: {
                        let a = px_idx_start_of_group;
                        let b = px_idx_start_of_group + 7 + $pad;
                        let target = &mut aux.obj_gradient[a..=b];

                        (alpha * -(g_xs[c] + g_ys[c]))
                            .div(g_norm)
                            .add_slice(target)
                            .store_select(target, mask);
                    }

                    '_for_shifted_right_1px_group: {
                        if group_at_right_edge {
                            let a = px_idx_start_of_group + 1;
                            let b = px_idx_start_of_group + 7 + $pad;
                            let target = &mut aux.obj_gradient[a..=b];

                            (alpha * g_xs[c])
                                .div(g_norm)
                                .add_short_slice(target)
                                .store_select(target, mask);
                        } else {
                            let a = px_idx_start_of_group + 1;
                            let b = px_idx_start_of_group + 8 + $pad;
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
                        let b = a + 7 + $pad;
                        let target = &mut aux.obj_gradient[a..=b];

                        (alpha * g_ys[c])
                            .div(g_norm)
                            .add_slice(target)
                            .store_select(target, mask);
                    }

                    // ===== store for use in tv2 =====
                    let a = px_idx_start_of_group;
                    let b = px_idx_start_of_group + 7 + $pad;

                    g_xs[c].write_to(&mut auxs[c].pixel_diff.x[a..=b]);
                    if !group_at_bottom_edge {
                        g_ys[c].write_to(&mut auxs[c].pixel_diff.y[a..=b]);
                    }
                }
            }
        }
    };
}

gen_func!(width: 8, pad: 0);
gen_func!(width: 16, pad: 8);
gen_func!(width: 32, pad: 24);
gen_func!(width: 64, pad: 56);
