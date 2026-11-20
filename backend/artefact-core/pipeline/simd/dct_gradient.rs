use std::{
    ops::{Div, Mul, Sub},
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
    let hf = coef.horizontal_samp_factor.u32();
    let vf = coef.vertical_samp_factor.u32();
    let mut blocks = cos.as_chunks::<64>().0.iter().zip(coef.dct_coefs.iter());

    // Iterate through each 8x8 block in the image
    for block_y in 0..coef.block_h {
        for block_x in 0..coef.block_w {
            let Some((cos_slice, &dct_coef)) = blocks.next() else {
                return;
            };

            // 8x8 block buffer
            let mut cosbs = [0.0; 64];

            f32x64::from_slice(cos_slice)
                .sub(dct_coef.mul(coef.quant_table))
                .div(coef.quant_table_squared)
                .write_to(&mut cosbs);

            // Apply inverse DCT to get spatial domain gradient
            idct8x8(&mut cosbs);

            // Distribute gradient to output buffer with upsampling
            for (in_y, cos_row) in cosbs.as_chunks::<8>().0.iter().enumerate() {
                let Ok(row_in_block) = u32::try_from(in_y) else {
                    continue;
                };
                for (in_x, &cos_val) in cos_row.iter().enumerate() {
                    let Ok(col_in_block) = u32::try_from(in_x) else {
                        continue;
                    };
                    let cx = block_x.saturating_mul(8).saturating_add(col_in_block);
                    let cy = block_y.saturating_mul(8).saturating_add(row_in_block);
                    // Apply sampling factors (upsampling): replicate each
                    // coefficient pixel across the component's subsampling block.
                    for sy in 0..vf {
                        for sx in 0..hf {
                            if let Some(idx) = cy
                                .checked_mul(vf)
                                .and_then(|v| v.checked_add(sy))
                                .and_then(|v| v.checked_mul(max_rounded_px_w))
                                .and_then(|v| v.checked_add(cx.checked_mul(hf)?.checked_add(sx)?))
                                .and_then(|v| usize::try_from(v).ok())
                                && let Some(slot) = obj_gradient.get_mut(idx)
                            {
                                *slot = alpha.mul_add(cos_val, *slot);
                            }
                        }
                    }
                }
            }
        }
    }
}
