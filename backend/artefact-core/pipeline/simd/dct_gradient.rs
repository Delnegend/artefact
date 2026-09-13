use std::{
    ops::{Div, Sub},
    simd::f32x64,
};

use super::{coef::SIMDCoef, traits::WriteTo};
use crate::utils::dct::idct8x8;

/// Gradient of the Discrete Cosine Transform (DCT) data-fidelity term: the
/// squared deviation of the current coefficients from the quantized originals,
/// back-projected to the pixel domain.
pub fn dct_gradient(
    max_rounded_px_w: u32,    // Maximum width after rounding to block size
    _max_rounded_px_h: u32,   // Maximum height after rounding to block size
    alpha: f32,               // Learning rate parameter
    coef: &SIMDCoef,          // JPEG coefficient data
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
            idct8x8(&mut cosbs);

            // Distribute gradient to output buffer with upsampling
            for in_y in 0..8 {
                for in_x in 0..8 {
                    let j = (in_y * 8 + in_x) as usize;
                    let cx = block_x * 8 + in_x;
                    let cy = block_y * 8 + in_y;

                    // Apply sampling factors (upsampling): replicate each
                    // coefficient pixel across the component's subsampling block.
                    let vf = coef.vertical_samp_factor.u32();
                    let hf = coef.horizontal_samp_factor.u32();
                    for sy in 0..vf {
                        for sx in 0..hf {
                            let idx = ((cy * vf + sy) * max_rounded_px_w + (cx * hf + sx)) as usize;
                            obj_gradient[idx] = alpha.mul_add(cosbs[j], obj_gradient[idx]);
                        }
                    }
                }
            }
        }
    }
}
