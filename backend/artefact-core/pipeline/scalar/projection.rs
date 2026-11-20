use super::ScalarCoef;
use crate::utils::{
    auxiliary::Aux,
    blocks::{from_blocks, to_blocks},
    dct::{dct8x8, idct8x8},
};

/// Project each 8x8 block onto its quantized Discrete Cosine Transform (DCT)
/// interval (scalar reference).
pub fn project_onto_box(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &ScalarCoef,
) {
    let resample = coef.rounded_px_w != max_rounded_px_w || coef.rounded_px_h != max_rounded_px_h;

    // downsample and keep the difference
    // more formally, decompose each subsampling block in the direction of our subsampling vector (a vector of ones)

    if resample {
        downsample_resample(max_rounded_px_w, max_rounded_px_h, aux, coef);
    }

    // Project onto DCT box
    to_blocks(
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

    let block_count = usize::try_from(coef.block_count).unwrap_or(0);
    for block in aux
        .pixel_diff
        .x
        .as_chunks_mut::<64>()
        .0
        .iter_mut()
        .take(block_count)
    {
        dct8x8(block);
    }

    // Clamp DCT coefficients
    for (block, dct_block) in aux
        .pixel_diff
        .x
        .as_chunks_mut::<64>()
        .0
        .iter_mut()
        .zip(coef.dct_coefs.as_chunks::<64>().0.iter())
        .take(block_count)
    {
        for ((val, &dct), &q) in block
            .iter_mut()
            .zip(dct_block.iter())
            .zip(coef.quant_table.iter())
        {
            let min = (dct - 0.5) * q;
            let max = (dct + 0.5) * q;
            *val = val.clamp(min, max);
        }
    }

    // Save a copy of the DCT values for `dct_gradient`
    aux.cos = aux.pixel_diff.x.clone();

    // add back the difference (orthogonal to our subsampling vector)
    for block in aux
        .pixel_diff
        .x
        .as_chunks_mut::<64>()
        .0
        .iter_mut()
        .take(block_count)
    {
        idct8x8(block);
    }

    from_blocks(
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
        upsample_resample(max_rounded_px_w, aux, coef);
    }
}

fn downsample_resample(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &ScalarCoef,
) {
    let v_samp = coef.vertical_samp_factor.u32();
    let h_samp = coef.horizontal_samp_factor.u32();
    let samp_area = f32::from(
        coef.horizontal_samp_factor
            .u8()
            .saturating_mul(coef.vertical_samp_factor.u8()),
    );
    for cy in 0..coef.rounded_px_h {
        for cx in 0..coef.rounded_px_w {
            let mut mean = 0.0;
            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);
                    debug_assert!(y < max_rounded_px_h && x < max_rounded_px_w);
                    if let Ok(idx) =
                        usize::try_from(y.saturating_mul(max_rounded_px_w).saturating_add(x))
                        && let Some(&val) = aux.fdata.get(idx)
                    {
                        mean += val;
                    }
                }
            }
            mean /= samp_area;

            debug_assert!(cx < coef.rounded_px_w && cy < coef.rounded_px_h);
            if let Ok(idx) =
                usize::try_from(cy.saturating_mul(coef.rounded_px_w).saturating_add(cx))
                && let Some(slot) = aux.pixel_diff.y.get_mut(idx)
            {
                *slot = mean;
            }

            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);

                    debug_assert!(y < max_rounded_px_h && x < max_rounded_px_w);
                    if let Ok(idx) =
                        usize::try_from(y.saturating_mul(max_rounded_px_w).saturating_add(x))
                        && let Some(slot) = aux.fdata.get_mut(idx)
                    {
                        *slot -= mean;
                    }
                }
            }
        }
    }
}

fn upsample_resample(max_rounded_px_w: u32, aux: &mut Aux, coef: &ScalarCoef) {
    let v_samp = coef.vertical_samp_factor.u32();
    let h_samp = coef.horizontal_samp_factor.u32();
    for cy in 0..coef.rounded_px_h {
        for cx in 0..coef.rounded_px_w {
            let mean = usize::try_from(cy.saturating_mul(coef.rounded_px_w).saturating_add(cx))
                .ok()
                .and_then(|idx| aux.pixel_diff.y.get(idx).copied())
                .unwrap_or(0.0);
            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);
                    if let Ok(idx) =
                        usize::try_from(y.saturating_mul(max_rounded_px_w).saturating_add(x))
                        && let Some(slot) = aux.fdata.get_mut(idx)
                    {
                        *slot += mean;
                    }
                }
            }
        }
    }
}
