use crate::{pipeline_scalar::coef::ScalarCoef, utils::dct::idct8x8s};

// Compute objective gradient for the distance of DCT coefficients from normal decoding
// N.B. destroys cos
pub fn compute_step_prob(
    max_rounded_px_w: u32,    // Maximum width after rounding to block size
    max_rounded_px_h: u32,    // Maximum height after rounding to block size
    alpha: f32,               // Learning rate parameter
    coef: &ScalarCoef,        // JPEG coefficient data
    cos: &[f32],              // Cosine transform data
    obj_gradient: &mut [f32], // Output gradient buffer
) {
    // Iterate through each 8x8 block in the image
    for block_y in 0..coef.block_h {
        for block_x in 0..coef.block_w {
            // Calculate block index and prepare cosine buffer
            let i = (block_y * coef.block_w + block_x) as usize;
            // 8x8 block buffer
            let mut cosbs: [f32; 64] = cos[i * 64..(i + 1) * 64]
                .try_into()
                .expect("Invalid cosine transform data length");

            // Process each coefficient in current block
            for (j, cosb) in cosbs.iter_mut().enumerate() {
                // Calculate difference from original DCT coefficients
                *cosb -= coef.dct_coefs[i * 64 + j] * coef.quant_table[j];

                // Calculate derivative for gradient
                *cosb /= (coef.quant_table[j]).powi(2);
            }

            // Apply inverse DCT to get spatial domain gradient
            idct8x8s(&mut cosbs);

            // Distribute gradient to output buffer with upsampling
            for in_y in 0..8 {
                for in_x in 0..8 {
                    let j = (in_y * 8 + in_x) as usize;
                    let cx = block_x * 8 + in_x;
                    let cy = block_y * 8 + in_y;

                    // Apply sampling factors (upsampling)
                    for sy in 0..coef.vertical_samp_factor.u32() {
                        for sx in 0..coef.horizontal_samp_factor.u32() {
                            let y = cy * coef.vertical_samp_factor.u32() + sy;
                            let x = cx * coef.horizontal_samp_factor.u32() + sx;

                            // Bounds checking
                            debug_assert!(y < max_rounded_px_h);
                            debug_assert!(x < max_rounded_px_w);

                            // Update gradient with scaled cosine value
                            obj_gradient[(y * max_rounded_px_w + x) as usize] += alpha * cosbs[j];
                        }
                    }
                }
            }
        }
    }
}
