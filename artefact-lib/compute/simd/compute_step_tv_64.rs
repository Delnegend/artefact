use std::simd::{cmp::SimdPartialEq, f32x64, StdFloat};

use crate::compute::aux::Aux;

/// This unfortunately slower than [`f32x8`].
///
/// [`f32x8`]: wide::f32x8
#[allow(unused)]
pub fn compute_step_tv_simd_64(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) {
    let alpha = 1.0 / (nchannel as f32).sqrt();

    for curr_row in (0..max_rounded_px_h).step_by(8) {
        for curr_row_px_idx in (0..max_rounded_px_w).step_by(8) {
            compute_step_tv_inner(
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

fn compute_step_tv_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_px_idx: u32,
    curr_row: u32,
    alpha: f32x64,
) {
    // a "group" = 8 consecutive pixels horizontally
    let px_idx_start_of_block = (curr_row * max_rounded_px_w + curr_row_px_idx) as usize;
    let block_at_right_edge = curr_row_px_idx as usize + 8 == max_rounded_px_w as usize;
    let block_at_bottom_edge = curr_row as usize + 8 == max_rounded_px_h as usize;

    let mut g_xs = [f32x64::splat(0.0); 3];
    let mut g_ys = [f32x64::splat(0.0); 3];

    let w = max_rounded_px_w as usize;

    // compute forward differences
    for c in 0..nchannel {
        let fdata = &auxs[c].fdata;

        // forward difference x
        g_xs[c] = if block_at_right_edge {
            let curr_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block;
                let b = px_idx_start_of_block + 6;
                tmp[0..=6].copy_from_slice(&fdata[a..=b]);
                tmp[8..=14].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=22].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=30].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=38].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=46].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=54].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=62].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            let shift_right_1px_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block + 1;
                let b = px_idx_start_of_block + 7;
                tmp[0..=6].copy_from_slice(&fdata[a..=b]);
                tmp[8..=14].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=22].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=30].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=38].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=46].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=54].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=62].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            shift_right_1px_block - curr_block
        } else {
            let curr_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block;
                let b = px_idx_start_of_block + 7;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=63].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            let shift_right_1px_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block + 1;
                let b = px_idx_start_of_block + 8;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=63].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            shift_right_1px_block - curr_block
        };

        // forward difference y
        g_ys[c] = if block_at_bottom_edge {
            let curr_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block;
                let b = px_idx_start_of_block + 7;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp
            };

            let shift_down_1px_block = {
                let mut tmp = f32x64::splat(0.0);

                let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
                let a = i;
                let b = i + 7;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp
            };

            shift_down_1px_block - curr_block
        } else {
            let curr_block = {
                let mut tmp = f32x64::splat(0.0);
                let a = px_idx_start_of_block;
                let b = px_idx_start_of_block + 7;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=63].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            let shift_down_1px_block = {
                let mut tmp = f32x64::splat(0.0);
                let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
                let a = i;
                let b = i + 7;
                tmp[0..=7].copy_from_slice(&fdata[a..=b]);
                tmp[8..=15].copy_from_slice(&fdata[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&fdata[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&fdata[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&fdata[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&fdata[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&fdata[a + w * 6..=b + w * 6]);
                tmp[56..=63].copy_from_slice(&fdata[a + w * 7..=b + w * 7]);
                tmp
            };

            shift_down_1px_block - curr_block
        };
    }

    // compute gradient normalization
    let g_norm = (0..nchannel)
        .map(|c| g_xs[c] * g_xs[c] + g_ys[c] * g_ys[c])
        .fold(f32x64::splat(0.0), |acc, x| acc + x)
        .sqrt();

    // compute derivatives
    for c in 0..nchannel {
        let obj = &mut auxs[c].obj_gradient;

        '_for_current_group: {
            let a = px_idx_start_of_block;
            let b = px_idx_start_of_block + 7;

            let original = {
                let mut tmp = f32x64::splat(0.0);
                tmp[0..=7].copy_from_slice(&obj[a..=b]);
                tmp[8..=15].copy_from_slice(&obj[a + w..=b + w]);
                tmp[16..=23].copy_from_slice(&obj[a + w * 2..=b + w * 2]);
                tmp[24..=31].copy_from_slice(&obj[a + w * 3..=b + w * 3]);
                tmp[32..=39].copy_from_slice(&obj[a + w * 4..=b + w * 4]);
                tmp[40..=47].copy_from_slice(&obj[a + w * 5..=b + w * 5]);
                tmp[48..=55].copy_from_slice(&obj[a + w * 6..=b + w * 6]);
                tmp[56..=63].copy_from_slice(&obj[a + w * 7..=b + w * 7]);
                tmp
            };
            let result = {
                let enable = g_norm.simd_ne(f32x64::splat(0.0));
                let mut tmp = [0.0; 64];
                (alpha * -(g_xs[c] + g_ys[c]) / g_norm).store_select(&mut tmp, enable);
                original + f32x64::from(tmp)
            };

            obj[a..=b].copy_from_slice(&result[0..=7]);
            obj[a + w..=b + w].copy_from_slice(&result[8..=15]);
            obj[a + w * 2..=b + w * 2].copy_from_slice(&result[16..=23]);
            obj[a + w * 3..=b + w * 3].copy_from_slice(&result[24..=31]);
            obj[a + w * 4..=b + w * 4].copy_from_slice(&result[32..=39]);
            obj[a + w * 5..=b + w * 5].copy_from_slice(&result[40..=47]);
            obj[a + w * 6..=b + w * 6].copy_from_slice(&result[48..=55]);
            obj[a + w * 7..=b + w * 7].copy_from_slice(&result[56..=63]);
        }

        '_for_shifted_right_1px_group: {
            if block_at_right_edge {
                let a = px_idx_start_of_block + 1;
                let b = px_idx_start_of_block + 7;

                let original = {
                    let mut tmp = f32x64::splat(0.0);
                    tmp[0..=6].copy_from_slice(&obj[a..=b]);
                    tmp[8..=14].copy_from_slice(&obj[a + w..=b + w]);
                    tmp[16..=22].copy_from_slice(&obj[a + w * 2..=b + w * 2]);
                    tmp[24..=30].copy_from_slice(&obj[a + w * 3..=b + w * 3]);
                    tmp[32..=38].copy_from_slice(&obj[a + w * 4..=b + w * 4]);
                    tmp[40..=46].copy_from_slice(&obj[a + w * 5..=b + w * 5]);
                    tmp[48..=54].copy_from_slice(&obj[a + w * 6..=b + w * 6]);
                    tmp[56..=62].copy_from_slice(&obj[a + w * 7..=b + w * 7]);
                    tmp
                };

                let result = {
                    let enable = g_norm.simd_ne(f32x64::splat(0.0));
                    let mut tmp = [0.0; 64];
                    (alpha * g_xs[c] / g_norm).store_select(&mut tmp, enable);
                    original + f32x64::from(tmp)
                };

                obj[a..=b].copy_from_slice(&result[0..=6]);
                obj[a + w..=b + w].copy_from_slice(&result[8..=14]);
                obj[a + w * 2..=b + w * 2].copy_from_slice(&result[16..=22]);
                obj[a + w * 3..=b + w * 3].copy_from_slice(&result[24..=30]);
                obj[a + w * 4..=b + w * 4].copy_from_slice(&result[32..=38]);
                obj[a + w * 5..=b + w * 5].copy_from_slice(&result[40..=46]);
                obj[a + w * 6..=b + w * 6].copy_from_slice(&result[48..=54]);
                obj[a + w * 7..=b + w * 7].copy_from_slice(&result[56..=62]);
            } else {
                let a = px_idx_start_of_block + 1;
                let b = px_idx_start_of_block + 8;

                let original = {
                    let mut tmp = f32x64::splat(0.0);
                    tmp[0..=7].copy_from_slice(&obj[a..=b]);
                    tmp[8..=15].copy_from_slice(&obj[a + w..=b + w]);
                    tmp[16..=23].copy_from_slice(&obj[a + w * 2..=b + w * 2]);
                    tmp[24..=31].copy_from_slice(&obj[a + w * 3..=b + w * 3]);
                    tmp[32..=39].copy_from_slice(&obj[a + w * 4..=b + w * 4]);
                    tmp[40..=47].copy_from_slice(&obj[a + w * 5..=b + w * 5]);
                    tmp[48..=55].copy_from_slice(&obj[a + w * 6..=b + w * 6]);
                    tmp[56..=63].copy_from_slice(&obj[a + w * 7..=b + w * 7]);
                    tmp
                };
                let result = {
                    let enable = g_norm.simd_ne(f32x64::splat(0.0));
                    let mut tmp = [0.0; 64];
                    (alpha * g_xs[c] / g_norm).store_select(&mut tmp, enable);
                    original + f32x64::from(tmp)
                };

                obj[a..=b].copy_from_slice(&result[0..=7]);
                obj[a + w..=b + w].copy_from_slice(&result[8..=15]);
                obj[a + w * 2..=b + w * 2].copy_from_slice(&result[16..=23]);
                obj[a + w * 3..=b + w * 3].copy_from_slice(&result[24..=31]);
                obj[a + w * 4..=b + w * 4].copy_from_slice(&result[32..=39]);
                obj[a + w * 5..=b + w * 5].copy_from_slice(&result[40..=47]);
                obj[a + w * 6..=b + w * 6].copy_from_slice(&result[48..=55]);
                obj[a + w * 7..=b + w * 7].copy_from_slice(&result[56..=63]);
            }
        }

        '_for_shifted_down_1px_group: {
            let i = ((curr_row + 1) * max_rounded_px_w + curr_row_px_idx) as usize;
            let a = i;
            let b = i + 7;

            if block_at_bottom_edge {
                let original = {
                    let mut tmp = f32x64::splat(0.0);
                    tmp[0..=7].copy_from_slice(&obj[a..=b]);
                    tmp[8..=15].copy_from_slice(&obj[a + w..=b + w]);
                    tmp[16..=23].copy_from_slice(&obj[a + w * 2..=b + w * 2]);
                    tmp[24..=31].copy_from_slice(&obj[a + w * 3..=b + w * 3]);
                    tmp[32..=39].copy_from_slice(&obj[a + w * 4..=b + w * 4]);
                    tmp[40..=47].copy_from_slice(&obj[a + w * 5..=b + w * 5]);
                    tmp[48..=55].copy_from_slice(&obj[a + w * 6..=b + w * 6]);
                    tmp
                };
                let result = {
                    let enable = g_norm.simd_ne(f32x64::splat(0.0));
                    let mut tmp = [0.0; 64];
                    (alpha * g_ys[c] / g_norm).store_select(&mut tmp, enable);
                    original + f32x64::from(tmp)
                };

                obj[a..=b].copy_from_slice(&result[0..=7]);
                obj[a + w..=b + w].copy_from_slice(&result[8..=15]);
                obj[a + w * 2..=b + w * 2].copy_from_slice(&result[16..=23]);
                obj[a + w * 3..=b + w * 3].copy_from_slice(&result[24..=31]);
                obj[a + w * 4..=b + w * 4].copy_from_slice(&result[32..=39]);
                obj[a + w * 5..=b + w * 5].copy_from_slice(&result[40..=47]);
                obj[a + w * 6..=b + w * 6].copy_from_slice(&result[48..=55]);
            } else {
                let original = {
                    let mut tmp = f32x64::splat(0.0);
                    tmp[0..=7].copy_from_slice(&obj[a..=b]);
                    tmp[8..=15].copy_from_slice(&obj[a + w..=b + w]);
                    tmp[16..=23].copy_from_slice(&obj[a + w * 2..=b + w * 2]);
                    tmp[24..=31].copy_from_slice(&obj[a + w * 3..=b + w * 3]);
                    tmp[32..=39].copy_from_slice(&obj[a + w * 4..=b + w * 4]);
                    tmp[40..=47].copy_from_slice(&obj[a + w * 5..=b + w * 5]);
                    tmp[48..=55].copy_from_slice(&obj[a + w * 6..=b + w * 6]);
                    tmp[56..=63].copy_from_slice(&obj[a + w * 7..=b + w * 7]);
                    tmp
                };
                let result = {
                    let enable = g_norm.simd_ne(f32x64::splat(0.0));
                    let mut tmp = [0.0; 64];
                    (alpha * g_ys[c] / g_norm).store_select(&mut tmp, enable);
                    original + f32x64::from(tmp)
                };

                obj[a..=b].copy_from_slice(&result[0..=7]);
                obj[a + w..=b + w].copy_from_slice(&result[8..=15]);
                obj[a + w * 2..=b + w * 2].copy_from_slice(&result[16..=23]);
                obj[a + w * 3..=b + w * 3].copy_from_slice(&result[24..=31]);
                obj[a + w * 4..=b + w * 4].copy_from_slice(&result[32..=39]);
                obj[a + w * 5..=b + w * 5].copy_from_slice(&result[40..=47]);
                obj[a + w * 6..=b + w * 6].copy_from_slice(&result[48..=55]);
                obj[a + w * 7..=b + w * 7].copy_from_slice(&result[56..=63]);
            }
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        let diff = &mut auxs[c].pixel_diff;

        let a = px_idx_start_of_block;
        let b = px_idx_start_of_block + 7;

        let g_x = g_xs[c].as_array();

        diff.x[a..=b].copy_from_slice(&g_x[0..=7]);
        diff.x[a + w..=b + w].copy_from_slice(&g_x[8..=15]);
        diff.x[a + w * 2..=b + w * 2].copy_from_slice(&g_x[16..=23]);
        diff.x[a + w * 3..=b + w * 3].copy_from_slice(&g_x[24..=31]);
        diff.x[a + w * 4..=b + w * 4].copy_from_slice(&g_x[32..=39]);
        diff.x[a + w * 5..=b + w * 5].copy_from_slice(&g_x[40..=47]);
        diff.x[a + w * 6..=b + w * 6].copy_from_slice(&g_x[48..=55]);
        diff.x[a + w * 7..=b + w * 7].copy_from_slice(&g_x[56..=63]);

        let g_y = g_ys[c].as_array();

        diff.y[a..=b].copy_from_slice(&g_y[0..=7]);
        diff.y[a + w..=b + w].copy_from_slice(&g_y[8..=15]);
        diff.y[a + w * 2..=b + w * 2].copy_from_slice(&g_y[16..=23]);
        diff.y[a + w * 3..=b + w * 3].copy_from_slice(&g_y[24..=31]);
        diff.y[a + w * 4..=b + w * 4].copy_from_slice(&g_y[32..=39]);
        diff.y[a + w * 5..=b + w * 5].copy_from_slice(&g_y[40..=47]);
        diff.y[a + w * 6..=b + w * 6].copy_from_slice(&g_y[48..=55]);
        diff.y[a + w * 7..=b + w * 7].copy_from_slice(&g_y[56..=63]);
    }
}
