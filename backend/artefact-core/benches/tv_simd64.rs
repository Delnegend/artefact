use std::ops::{Add, Div, Mul, Neg, Sub};
use std::simd::{StdFloat, cmp::SimdPartialEq, f32x64};

use artefact_core::Aux;

/// This unfortunately slower than [`f32x8`].
///
/// [`f32x8`]: std::simd::f32x8
pub fn tv_gradient_simd64(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let nchan_f32 = f32::from(u16::try_from(nchannel).unwrap_or(1));
    let alpha = 1.0 / nchan_f32.sqrt();

    for curr_row in (0..max_rounded_px_h).step_by(8) {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            tv_gradient_px(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                curr_row_px_idx,
                curr_row,
                f32x64::splat(alpha),
            );
        }
    }
}

fn load_block(src: &[f32], start: usize, stride: usize, rows: usize, cols: usize) -> f32x64 {
    let mut tmp = f32x64::splat(0.0);
    for r in 0..rows {
        let src_off = start.saturating_add(r.saturating_mul(stride));
        let dst_off = r.saturating_mul(8);
        if let (Some(dst_slice), Some(src_slice)) = (
            tmp.as_mut_array()
                .get_mut(dst_off..dst_off.saturating_add(cols)),
            src.get(src_off..src_off.saturating_add(cols)),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
    }
    tmp
}

fn store_block(
    dst: &mut [f32],
    start: usize,
    stride: usize,
    rows: usize,
    cols: usize,
    val: &f32x64,
) {
    let arr = val.as_array();
    for r in 0..rows {
        let dst_off = start.saturating_add(r.saturating_mul(stride));
        let src_off = r.saturating_mul(8);
        if let (Some(dst_slice), Some(src_slice)) = (
            dst.get_mut(dst_off..dst_off.saturating_add(cols)),
            arr.get(src_off..src_off.saturating_add(cols)),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
    }
}

fn tv_gradient_px(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_px_idx: u32,
    curr_row: u32,
    alpha: f32x64,
) {
    let px_idx_start_of_block = usize::try_from(
        curr_row
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_px_idx),
    )
    .unwrap_or(0);
    let w = usize::try_from(max_rounded_px_w).unwrap_or(0);
    let h = usize::try_from(max_rounded_px_h).unwrap_or(0);
    let block_at_right_edge = usize::try_from(curr_row_px_idx)
        .unwrap_or(0)
        .saturating_add(8)
        == w;
    let block_at_bottom_edge = usize::try_from(curr_row).unwrap_or(0).saturating_add(8) == h;

    let mut grad_x = [f32x64::splat(0.0); 3];
    let mut grad_y = [f32x64::splat(0.0); 3];

    for c in 0..nchannel {
        let Some(aux) = auxs.get(c) else {
            continue;
        };
        let fdata = &aux.fdata;

        let cols = if block_at_right_edge { 7 } else { 8 };
        let curr_x = load_block(fdata, px_idx_start_of_block, w, 8, cols);
        let shift_x = load_block(fdata, px_idx_start_of_block.saturating_add(1), w, 8, cols);
        if let Some(slot) = grad_x.get_mut(c) {
            *slot = Sub::sub(shift_x, curr_x);
        }

        let rows = if block_at_bottom_edge { 7 } else { 8 };
        let curr_y = load_block(fdata, px_idx_start_of_block, w, rows, 8);
        let down_start = usize::try_from(
            curr_row
                .saturating_add(1)
                .saturating_mul(max_rounded_px_w)
                .saturating_add(curr_row_px_idx),
        )
        .unwrap_or(0);
        let shift_y = load_block(fdata, down_start, w, rows, 8);
        if let Some(slot) = grad_y.get_mut(c) {
            *slot = Sub::sub(shift_y, curr_y);
        }
    }

    let g_norm = (0..nchannel)
        .filter_map(|c| {
            let gx = grad_x.get(c)?;
            let gy = grad_y.get(c)?;
            Some(Add::add(Mul::mul(*gx, *gx), Mul::mul(*gy, *gy)))
        })
        .fold(f32x64::splat(0.0), Add::add)
        .sqrt();

    let enable = g_norm.simd_ne(f32x64::splat(0.0));

    for c in 0..nchannel {
        let (Some(aux), Some(&gx), Some(&gy)) = (auxs.get_mut(c), grad_x.get(c), grad_y.get(c))
        else {
            continue;
        };
        let obj = &mut aux.obj_gradient;

        {
            let original = load_block(obj, px_idx_start_of_block, w, 8, 8);
            let mut tmp = [0.0; 64];
            Div::div(Mul::mul(alpha, Neg::neg(Add::add(gx, gy))), g_norm)
                .store_select(&mut tmp, enable);
            let result = Add::add(original, f32x64::from(tmp));
            store_block(obj, px_idx_start_of_block, w, 8, 8, &result);
        }

        {
            let cols = if block_at_right_edge { 7 } else { 8 };
            let start = px_idx_start_of_block.saturating_add(1);
            let original = load_block(obj, start, w, 8, cols);
            let mut tmp = [0.0; 64];
            Div::div(Mul::mul(alpha, gx), g_norm).store_select(&mut tmp, enable);
            let result = Add::add(original, f32x64::from(tmp));
            store_block(obj, start, w, 8, cols, &result);
        }

        {
            let down_start = usize::try_from(
                curr_row
                    .saturating_add(1)
                    .saturating_mul(max_rounded_px_w)
                    .saturating_add(curr_row_px_idx),
            )
            .unwrap_or(0);
            let rows = if block_at_bottom_edge { 7 } else { 8 };
            let original = load_block(obj, down_start, w, rows, 8);
            let mut tmp = [0.0; 64];
            Div::div(Mul::mul(alpha, gy), g_norm).store_select(&mut tmp, enable);
            let result = Add::add(original, f32x64::from(tmp));
            store_block(obj, down_start, w, rows, 8, &result);
        }
    }

    for c in 0..nchannel {
        let (Some(aux), Some(gx), Some(gy)) = (auxs.get_mut(c), grad_x.get(c), grad_y.get(c))
        else {
            continue;
        };
        let diff = &mut aux.pixel_diff;
        store_block(&mut diff.x, px_idx_start_of_block, w, 8, 8, gx);
        store_block(&mut diff.y, px_idx_start_of_block, w, 8, 8, gy);
    }
}
