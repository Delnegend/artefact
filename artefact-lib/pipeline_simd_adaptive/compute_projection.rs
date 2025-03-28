use std::simd::{f32x64, num::SimdFloat};

use zune_jpeg::sample_factor::SampleFactor;

use super::coef::SIMDAdaptiveCoef;
use crate::utils::{
    aux::Aux,
    boxing::{boxing, unboxing},
    dct::{dct8x8s, idct8x8s},
    traits::WriteTo,
};

pub fn compute_projection(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &SIMDAdaptiveCoef,
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

                mean /=
                    f32::from(coef.horizontal_samp_factor.u8() * coef.vertical_samp_factor.u8());

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
        if resample {
            &aux.pixel_diff.y
        } else {
            &aux.fdata
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
        let a = i * 64;
        let b = a + 63;
        let old = &mut aux.pixel_diff.x[a..=b];

        let max = coef.dequant_dct_coefs_max[i];
        let min = coef.dequant_dct_coefs_min[i];

        f32x64::from_slice(old).simd_clamp(min, max).write_to(old);
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
        if resample {
            aux.pixel_diff.y.as_mut()
        } else {
            aux.fdata.as_mut()
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
