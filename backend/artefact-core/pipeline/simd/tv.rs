use std::{
    ops::Div,
    simd::{Simd, StdFloat, cmp::SimdPartialEq},
};

use super::{
    adaptive_width::{AdaptiveWidth, dispatch_run},
    run::{load, load_or_zero, run_mut},
    traits::{AddSlice, WriteTo},
};
use crate::utils::auxiliary::Aux;

/// First-order Total Variation (TV) gradient: each run in `adaptive_widths` is
/// processed with the matching `Simd<f32, N>` lane width.
pub fn tv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    adaptive_widths: &[AdaptiveWidth],
) {
    for curr_row in 0..max_rounded_px_h {
        for &adaptive_width in adaptive_widths {
            dispatch_run!(
                adaptive_width,
                tv_run,
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs;
                curr_row
            );
        }
    }
}

/// First-order Total Variation (TV) forward-difference gradient for one run of
/// `N` lanes.
fn tv_run<const N: usize>(
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
    let start = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let group_at_right_edge = curr_row_px_idx as usize + 8 + pad == max_rounded_px_w as usize;
    let group_at_bottom_edge = curr_row + 1 == max_rounded_px_h;
    let below = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;

    let mut g_xs = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_ys = [Simd::<f32, N>::splat(0.0); 3];

    // compute forward differences
    for c in 0..nchannel {
        let aux = &auxs[c];

        // forward difference x
        g_xs[c] = if group_at_right_edge {
            load_or_zero::<N>(&aux.fdata, start + 1, N - 1)
                - load_or_zero::<N>(&aux.fdata, start, N - 1)
        } else {
            load::<N>(&aux.fdata, start + 1) - load::<N>(&aux.fdata, start)
        };

        // forward difference y
        if !group_at_bottom_edge {
            g_ys[c] = load::<N>(&aux.fdata, below) - load::<N>(&aux.fdata, start);
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
            let target = run_mut(&mut aux.obj_gradient, start, N);

            (alpha * -(g_xs[c] + g_ys[c]))
                .div(g_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        {
            if group_at_right_edge {
                let target = run_mut(&mut aux.obj_gradient, start + 1, N - 1);

                (alpha * g_xs[c])
                    .div(g_norm)
                    .add_short_slice(target)
                    .store_select(target, mask);
            } else {
                let target = run_mut(&mut aux.obj_gradient, start + 1, N);

                (alpha * g_xs[c])
                    .div(g_norm)
                    .add_slice(target)
                    .store_select(target, mask);
            }
        }

        // for the group below the current group
        if !group_at_bottom_edge {
            let target = run_mut(&mut aux.obj_gradient, below, N);

            (alpha * g_ys[c])
                .div(g_norm)
                .add_slice(target)
                .store_select(target, mask);
        }

        // ===== store for the second-order TGV pass =====
        g_xs[c].write_to(run_mut(&mut aux.pixel_diff.x, start, N));
        if !group_at_bottom_edge {
            g_ys[c].write_to(run_mut(&mut aux.pixel_diff.y, start, N));
        }
    }
}
