use std::simd::{f32x8, num::SimdFloat};

use zune_jpeg::sample_factor::SampleFactor;

use crate::{
    compute::aux::Aux,
    jpeg::Coefficient,
    utils::{
        boxing::{boxing, unboxing},
        dct::{dct8x8s, idct8x8s},
    },
};

pub fn compute_projection_simd_std(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &Coefficient,
) {
    let resample = coef.rounded_px_w != max_rounded_px_w || coef.rounded_px_h != max_rounded_px_h;

    // downsample and keep the difference
    // more formally, decompose each subsampling block in the direction of our subsampling vector (a vector of ones)
    if resample {
        for cy in 0..coef.rounded_px_h {
            for cx in 0..coef.rounded_px_w {
                let mut mean = match (coef.vertical_samp_factor, coef.horizontal_samp_factor) {
                    (SampleFactor::One, SampleFactor::One) => {
                        aux.fdata[(cy * max_rounded_px_w + cx) as usize]
                    }
                    (SampleFactor::One, SampleFactor::Two) => {
                        aux.fdata[(cy * max_rounded_px_w + cx * 2) as usize]
                            + aux.fdata[(cy * max_rounded_px_w + cx * 2 + 1) as usize]
                    }
                    (SampleFactor::Two, SampleFactor::One) => {
                        aux.fdata[(cy * 2 * max_rounded_px_w + cx) as usize]
                            + aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx) as usize]
                    }
                    (SampleFactor::Two, SampleFactor::Two) => {
                        aux.fdata[(cy * 2 * max_rounded_px_w + cx * 2) as usize]
                            + aux.fdata[(cy * 2 * max_rounded_px_w + cx * 2 + 1) as usize]
                            + aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx * 2) as usize]
                            + aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx * 2 + 1) as usize]
                    }
                };

                mean /= (coef.horizontal_samp_factor.u8() * coef.vertical_samp_factor.u8()) as f32;

                debug_assert!(cx < coef.rounded_px_w && cy < coef.rounded_px_h);
                aux.pixel_diff.y[(cy * coef.rounded_px_w + cx) as usize] = mean;

                match (coef.vertical_samp_factor, coef.horizontal_samp_factor) {
                    (SampleFactor::One, SampleFactor::One) => {
                        aux.fdata[(cy * max_rounded_px_w + cx) as usize] -= mean;
                    }
                    (SampleFactor::One, SampleFactor::Two) => {
                        aux.fdata[(cy * max_rounded_px_w + cx * 2) as usize] -= mean;
                        aux.fdata[(cy * max_rounded_px_w + cx * 2 + 1) as usize] -= mean;
                    }
                    (SampleFactor::Two, SampleFactor::One) => {
                        aux.fdata[(cy * 2 * max_rounded_px_w + cx) as usize] -= mean;
                        aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx) as usize] -= mean;
                    }
                    (SampleFactor::Two, SampleFactor::Two) => {
                        aux.fdata[(cy * 2 * max_rounded_px_w + cx * 2) as usize] -= mean;
                        aux.fdata[(cy * 2 * max_rounded_px_w + cx * 2 + 1) as usize] -= mean;
                        aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx * 2) as usize] -= mean;
                        aux.fdata[((cy * 2 + 1) * max_rounded_px_w + cx * 2 + 1) as usize] -= mean;
                    }
                }
            }
        }
    }

    // Project onto DCT box
    boxing(
        match resample {
            true => &aux.pixel_diff.y,
            false => &aux.fdata,
        },
        aux.pixel_diff.x.as_mut(),
        coef.rounded_px_w,
        coef.rounded_px_h,
        coef.block_w,
        coef.block_h,
    );

    for i in 0..coef.block_count as usize {
        dct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    // Clamp DCT coefficients
    for i in 0..coef.block_count as usize {
        for j in 0..8 {
            let idx = i * 64 + j * 8;
            let target = &mut aux.pixel_diff.x[idx..idx + 8];

            let max = coef.dequant_dct_coefs_max[i * 8 + j];
            let min = coef.dequant_dct_coefs_min[i * 8 + j];

            target.copy_from_slice(
                &f32x8::from_slice(target)
                    .simd_min(max)
                    .simd_max(min)
                    .to_array(),
            );
        }
    }

    // Save a copy of the DCT values for step_prob
    aux.cos = aux.pixel_diff.x.clone();

    // add back the difference (orthogonal to our subsampling vector)
    for i in 0..coef.block_count as usize {
        idct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    unboxing(
        &aux.pixel_diff.x,
        match resample {
            true => aux.pixel_diff.y.as_mut(),
            false => aux.fdata.as_mut(),
        },
        coef.rounded_px_w,
        coef.rounded_px_h,
        coef.block_w,
        coef.block_h,
    );

    // Add back the difference
    if resample {
        for px_row in 0..coef.rounded_px_h {
            for row_idx in 0..coef.rounded_px_w {
                let mean = aux.pixel_diff.y[(px_row * coef.rounded_px_w + row_idx) as usize];
                match (coef.vertical_samp_factor, coef.horizontal_samp_factor) {
                    (SampleFactor::One, SampleFactor::One) => {
                        aux.fdata[(px_row * max_rounded_px_w + row_idx) as usize] += mean;
                    }
                    (SampleFactor::One, SampleFactor::Two) => {
                        aux.fdata[(px_row * max_rounded_px_w + row_idx * 2) as usize] += mean;
                        aux.fdata[(px_row * max_rounded_px_w + row_idx * 2 + 1) as usize] += mean;
                    }
                    (SampleFactor::Two, SampleFactor::One) => {
                        aux.fdata[(px_row * 2 * max_rounded_px_w + row_idx) as usize] += mean;
                        aux.fdata[((px_row * 2 + 1) * max_rounded_px_w + row_idx) as usize] += mean;
                    }
                    (SampleFactor::Two, SampleFactor::Two) => {
                        aux.fdata[(px_row * 2 * max_rounded_px_w + row_idx * 2) as usize] += mean;
                        aux.fdata[(px_row * 2 * max_rounded_px_w + row_idx * 2 + 1) as usize] +=
                            mean;
                        aux.fdata[((px_row * 2 + 1) * max_rounded_px_w + row_idx * 2) as usize] +=
                            mean;
                        aux.fdata
                            [((px_row * 2 + 1) * max_rounded_px_w + row_idx * 2 + 1) as usize] +=
                            mean;
                    }
                }
            }
        }
    }
}
