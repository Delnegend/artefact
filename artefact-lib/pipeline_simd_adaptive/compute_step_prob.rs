use std::{
    ops::{Div, Sub},
    simd::f32x64,
};

use zune_jpeg::sample_factor::SampleFactor;

use super::coef::SIMDAdaptiveCoef;
use crate::utils::{dct::idct8x8s, traits::WriteTo};

// Compute objective gradient for the distance of DCT coefficients from normal decoding
// N.B. destroys cos
#[allow(unused_variables)]
pub fn compute_step_prob(
    max_rounded_px_w: u32,    // Maximum width after rounding to block size
    max_rounded_px_h: u32,    // Maximum height after rounding to block size
    alpha: f32,               // Learning rate parameter
    coef: &SIMDAdaptiveCoef,  // JPEG coefficient data
    cos: &[f32],              // Cosine transform data
    obj_gradient: &mut [f32], // Output gradient buffer
) {
    // Iterate through each 8x8 block in the image
    for block_y in 0..coef.block_h {
        for block_x in 0..coef.block_w {
            // Calculate block index and prepare cosine buffer
            let i = (block_y * coef.block_w + block_x) as usize;

            // 8x8 block buffer
            let mut cosbs = [0.0; 64];

            f32x64::from_slice(&cos[i * 64..(i + 1) * 64])
                .sub(coef.dct_coefs[i] * coef.quant_table)
                .div(coef.quant_table_squared)
                .write_to(&mut cosbs);

            // Apply inverse DCT to get spatial domain gradient
            idct8x8s(&mut cosbs);

            // Distribute gradient to output buffer with upsampling
            for in_y in 0..8 {
                for in_x in 0..8 {
                    let j = (in_y * 8 + in_x) as usize;
                    let cx = block_x * 8 + in_x;
                    let cy = block_y * 8 + in_y;

                    // Apply sampling factors (upsampling)
                    match (coef.vertical_samp_factor, coef.horizontal_samp_factor) {
                        (SampleFactor::One, SampleFactor::One) => {
                            obj_gradient[(cy * max_rounded_px_w + cx) as usize] += alpha * cosbs[j];
                        }
                        (SampleFactor::One, SampleFactor::Two) => {
                            obj_gradient[(cy * max_rounded_px_w + cx * 2) as usize] +=
                                alpha * cosbs[j];
                            obj_gradient[(cy * max_rounded_px_w + cx * 2 + 1) as usize] +=
                                alpha * cosbs[j];
                        }
                        (SampleFactor::Two, SampleFactor::One) => {
                            obj_gradient[(cy * 2 * max_rounded_px_w + cx) as usize] +=
                                alpha * cosbs[j];
                            obj_gradient[((cy * 2 + 1) * max_rounded_px_w + cx) as usize] +=
                                alpha * cosbs[j];
                        }
                        (SampleFactor::Two, SampleFactor::Two) => {
                            obj_gradient[(cy * 2 * max_rounded_px_w + cx * 2) as usize] +=
                                alpha * cosbs[j];
                            obj_gradient[(cy * 2 * max_rounded_px_w + cx * 2 + 1) as usize] +=
                                alpha * cosbs[j];
                            obj_gradient[((cy * 2 + 1) * max_rounded_px_w + cx * 2) as usize] +=
                                alpha * cosbs[j];
                            obj_gradient
                                [((cy * 2 + 1) * max_rounded_px_w + cx * 2 + 1) as usize] +=
                                alpha * cosbs[j];
                        }
                    }
                }
            }
        }
    }
}
