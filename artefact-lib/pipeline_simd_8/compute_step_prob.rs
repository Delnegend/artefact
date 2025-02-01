use crate::{
    pipeline_simd_8::{f32x8, SIMD8Coef},
    utils::{
        dct::idct8x8s,
        traits::{FromSlice, WriteTo},
    },
};
use zune_jpeg::sample_factor::SampleFactor;

// Compute objective gradient for the distance of DCT coefficients from normal decoding
// N.B. destroys cos
#[allow(unused_variables)]
pub fn compute_step_prob(
    max_rounded_px_w: u32,    // Maximum width after rounding to block size
    max_rounded_px_h: u32,    // Maximum height after rounding to block size
    alpha: f32,               // Learning rate parameter
    coef: &SIMD8Coef,         // JPEG coefficient data
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
            for j in 0..8 {
                let target = &mut cosbs[j * 8..j * 8 + 8];
                let original = f32x8::from_slc(target);

                let update_a = coef.dct_coefs[i * 8 + j] * coef.quant_table[j];
                let update_b = coef.quant_table_squared[j];

                ((original - update_a) / update_b).write_to(target);
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
