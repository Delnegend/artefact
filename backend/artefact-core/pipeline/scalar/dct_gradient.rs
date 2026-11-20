use crate::pipeline::scalar::coef::ScalarCoef;
use crate::utils::dct::idct8x8;

/// Objective gradient of the Discrete Cosine Transform (DCT) data-fidelity term
/// (the distance of the current coefficients from the quantized originals).
///
/// N.B. destroys `cos`.
pub fn dct_gradient(
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
            let i = usize::try_from(block_y.saturating_mul(coef.block_w).saturating_add(block_x))
                .unwrap_or(0);
            let Some(cos_slice) = cos.as_chunks::<64>().0.get(i) else {
                continue;
            };
            let Some(dct_slice) = coef.dct_coefs.as_chunks::<64>().0.get(i) else {
                continue;
            };
            let mut cosbs: [f32; 64] = std::array::from_fn(|j| {
                let c = cos_slice.get(j).copied().unwrap_or(0.0);
                let d = dct_slice.get(j).copied().unwrap_or(0.0);
                let q = coef.quant_table.get(j).copied().unwrap_or(1.0);
                let diff = d.mul_add(-q, c);
                diff / q.powi(2)
            });
            // Apply inverse DCT to get spatial domain gradient
            idct8x8(&mut cosbs);

            // Distribute gradient to output buffer with upsampling
            for (in_y, cos_row) in cosbs.as_chunks::<8>().0.iter().enumerate() {
                let Some(in_y_u32) = u32::try_from(in_y).ok() else {
                    continue;
                };
                for (in_x, &cos_val) in cos_row.iter().enumerate() {
                    let Ok(x_in_block) = u32::try_from(in_x) else {
                        continue;
                    };
                    let cx = block_x.saturating_mul(8).saturating_add(x_in_block);
                    let cy = block_y.saturating_mul(8).saturating_add(in_y_u32);

                    // Apply sampling factors (upsampling)
                    let v_samp = coef.vertical_samp_factor.u32();
                    let h_samp = coef.horizontal_samp_factor.u32();
                    for sy in 0..v_samp {
                        for sx in 0..h_samp {
                            let y = cy.saturating_mul(v_samp).saturating_add(sy);
                            let x = cx.saturating_mul(h_samp).saturating_add(sx);

                            // Bounds checking
                            debug_assert!(y < max_rounded_px_h);
                            debug_assert!(x < max_rounded_px_w);

                            if let Ok(idx) = usize::try_from(
                                y.saturating_mul(max_rounded_px_w).saturating_add(x),
                            ) && let Some(slot) = obj_gradient.get_mut(idx)
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
