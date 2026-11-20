use std::ops::{Add, Mul, Neg, Sub};
use std::simd::{StdFloat, f32x8};

use rayon::prelude::*;

use artefact_core::{Aux, SafeDiv, WriteTo};

/// A slower, rayon-parallelized variant of the first-order Total Variation (TV)
/// gradient, kept as a benchmark reference.
pub fn tv_gradient_par(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let nchan_f32 = f32::from(u16::try_from(nchannel).unwrap_or(1));
    let alpha = f32x8::splat(1.0 / nchan_f32.sqrt());
    let max_rounded_px_count =
        usize::try_from(max_rounded_px_w.saturating_mul(max_rounded_px_h)).unwrap_or(0);
    let group_count = max_rounded_px_count / 8;

    let chans_forward_diffs = (0..nchannel)
        .into_par_iter()
        .map(|c| {
            let mut grad_x = vec![f32x8::splat(0.0); group_count];
            let mut grad_y = vec![f32x8::splat(0.0); group_count];

            if let Some(aux) = auxs.get(c) {
                for curr_row in 0..max_rounded_px_h {
                    for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
                        compute_forward_differents(
                            max_rounded_px_w,
                            max_rounded_px_h,
                            aux,
                            curr_row_px_idx,
                            curr_row,
                            &mut grad_x,
                            &mut grad_y,
                        );
                    }
                }
            }

            (grad_x, grad_y)
        })
        .collect::<Vec<_>>();

    let mut g_norm = vec![f32x8::splat(0.0); group_count];
    for (group_idx, norm_slot) in g_norm.iter_mut().enumerate() {
        let mut acc = f32x8::splat(0.0);
        for (grad_x, grad_y) in &chans_forward_diffs {
            let gx = grad_x.get(group_idx).copied().unwrap_or_default();
            let gy = grad_y.get(group_idx).copied().unwrap_or_default();
            acc = Add::add(acc, Add::add(Mul::mul(gx, gx), Mul::mul(gy, gy)));
        }
        *norm_slot = acc.sqrt();
    }

    auxs.par_iter_mut()
        .zip(chans_forward_diffs)
        .for_each(|(aux, (grad_x, grad_y))| {
            for curr_row in 0..max_rounded_px_h {
                for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
                    let px_idx_start_of_group = usize::try_from(
                        curr_row
                            .saturating_mul(max_rounded_px_w)
                            .saturating_add(curr_row_px_idx),
                    )
                    .unwrap_or(0);
                    let curr_group_idx = usize::try_from(
                        curr_row
                            .saturating_mul(max_rounded_px_w)
                            .wrapping_div(8)
                            .saturating_add(curr_row_px_idx / 8),
                    )
                    .unwrap_or(0);

                    let gx = grad_x.get(curr_group_idx).copied().unwrap_or_default();
                    let gy = grad_y.get(curr_group_idx).copied().unwrap_or_default();
                    let norm = g_norm.get(curr_group_idx).copied().unwrap_or_default();

                    compute_derivatives(
                        aux,
                        &DerivParams {
                            curr_row,
                            curr_row_px_idx,
                            max_rounded_px_w,
                            max_rounded_px_h,
                            curr_px_idx_start_of_group: px_idx_start_of_group,
                            gx,
                            gy,
                            alpha,
                            g_norm: norm,
                        },
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
    grad_x: &mut [f32x8],
    grad_y: &mut [f32x8],
) {
    let curr_px_idx_start_of_group = usize::try_from(
        curr_row
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_px_idx),
    )
    .unwrap_or(0);
    let group_at_right_edge = curr_row_px_idx.saturating_add(8) >= max_rounded_px_w;
    let group_at_bottom_edge = curr_row == max_rounded_px_h.saturating_sub(1);
    let group_idx = usize::try_from(
        curr_row
            .saturating_mul(max_rounded_px_w)
            .wrapping_div(8)
            .saturating_add(curr_row_px_idx / 8),
    )
    .unwrap_or(0);

    let gx_val = if group_at_right_edge {
        // groups_at_right_edge.insert(curr_px_idx_start_of_group);

        // only handle 7 consecutive pixels because the last one is at the
        // edge, and there's no more pixel to the right for us to calculate
        // the difference with

        let curr_group = aux
            .fdata
            .get(curr_px_idx_start_of_group..=curr_px_idx_start_of_group.saturating_add(6))
            .map_or_else(f32x8::default, f32x8::load_or_default);
        let shift_right_1px_group = aux
            .fdata
            .get(
                curr_px_idx_start_of_group.saturating_add(1)
                    ..=curr_px_idx_start_of_group.saturating_add(7),
            )
            .map_or_else(f32x8::default, f32x8::load_or_default);

        Sub::sub(shift_right_1px_group, curr_group)
    } else {
        let curr_group = aux
            .fdata
            .get(curr_px_idx_start_of_group..=curr_px_idx_start_of_group.saturating_add(7))
            .map_or_else(f32x8::default, f32x8::from_slice);
        let shift_right_1px_group = aux
            .fdata
            .get(
                curr_px_idx_start_of_group.saturating_add(1)
                    ..=curr_px_idx_start_of_group.saturating_add(8),
            )
            .map_or_else(f32x8::default, f32x8::from_slice);

        Sub::sub(shift_right_1px_group, curr_group)
    };
    if let Some(slot) = grad_x.get_mut(group_idx) {
        *slot = gx_val;
    }

    // forward difference y
    if !group_at_bottom_edge {
        let curr_group = aux
            .fdata
            .get(curr_px_idx_start_of_group..=curr_px_idx_start_of_group.saturating_add(7))
            .map_or_else(f32x8::default, f32x8::from_slice);

        let shift_down_1px_group_idx = usize::try_from(
            curr_row
                .saturating_add(1)
                .saturating_mul(max_rounded_px_w)
                .saturating_add(curr_row_px_idx),
        )
        .unwrap_or(0);
        let shift_down_1px_group = aux
            .fdata
            .get(shift_down_1px_group_idx..=shift_down_1px_group_idx.saturating_add(7))
            .map_or_else(f32x8::default, f32x8::from_slice);

        if let Some(slot) = grad_y.get_mut(group_idx) {
            *slot = Sub::sub(shift_down_1px_group, curr_group);
        }
    }
}

struct DerivParams {
    curr_row: u32,
    curr_row_px_idx: u32,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    curr_px_idx_start_of_group: usize,
    gx: f32x8,
    gy: f32x8,
    alpha: f32x8,
    g_norm: f32x8,
}

fn compute_derivatives(aux: &mut Aux, p: &DerivParams) {
    let group_at_right_edge = p.curr_row_px_idx.saturating_add(8) >= p.max_rounded_px_w;
    let group_at_bottom_edge = p.curr_row == p.max_rounded_px_h.saturating_sub(1);
    let start = p.curr_px_idx_start_of_group;

    if let Some(target) = aux.obj_gradient.get_mut(start..=start.saturating_add(7)) {
        let original = f32x8::from_slice(target);
        let update = Mul::mul(p.alpha, Neg::neg(Add::add(p.gx, p.gy))).safe_div(p.g_norm);
        Add::add(original, update).write_to(target);
    }

    {
        let update = Mul::mul(p.alpha, p.gx).safe_div(p.g_norm);

        if group_at_right_edge {
            if let Some(target) = aux
                .obj_gradient
                .get_mut(start.saturating_add(1)..=start.saturating_add(7))
            {
                let original = f32x8::load_or_default(target);
                Add::add(original, update).write_partial_to(target, 0..=6);
            }
        } else if let Some(target) = aux
            .obj_gradient
            .get_mut(start.saturating_add(1)..=start.saturating_add(8))
        {
            let original = f32x8::from_slice(target);
            Add::add(original, update).write_to(target);
        }
    }

    if !group_at_bottom_edge {
        let down_start = usize::try_from(
            p.curr_row
                .saturating_add(1)
                .saturating_mul(p.max_rounded_px_w)
                .saturating_add(p.curr_row_px_idx),
        )
        .unwrap_or(0);

        if let Some(target) = aux
            .obj_gradient
            .get_mut(down_start..=down_start.saturating_add(7))
        {
            let original = f32x8::from_slice(target);
            let update = Mul::mul(p.alpha, p.gy).safe_div(p.g_norm);
            Add::add(original, update).write_to(target);
        }
    }

    if let Some(target) = aux.pixel_diff.x.get_mut(start..=start.saturating_add(7)) {
        p.gx.write_to(target);
    }

    if !group_at_bottom_edge
        && let Some(target) = aux.pixel_diff.y.get_mut(start..=start.saturating_add(7))
    {
        p.gy.write_to(target);
    }
}
