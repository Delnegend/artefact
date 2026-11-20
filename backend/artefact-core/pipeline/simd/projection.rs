use super::coef::Coef;
use crate::utils::{
    auxiliary::Aux,
    blocks::{from_blocks, to_blocks},
    dct::{dct8x8, idct8x8},
};

/// Project each 8x8 block onto its quantized Discrete Cosine Transform (DCT)
/// interval: resample to the subsampling grid, DCT, clamp into the quantization
/// box, then inverse-DCT back to the pixel domain.
pub fn project_onto_box<C>(max_w: u32, max_h: u32, aux: &mut Aux, coef: &C)
where
    C: Coef,
{
    let resample = coef.rounded_px_w() != max_w || coef.rounded_px_h() != max_h;

    if resample {
        downsample_mean(max_w, max_h, aux, coef);
    }

    to_blocks(
        if resample {
            &aux.pixel_diff.y
        } else {
            &aux.fdata
        },
        aux.pixel_diff.x.as_mut(),
        coef.rounded_px_w(),
        coef.rounded_px_h(),
        coef.block_w(),
        coef.block_h(),
    );

    let block_count = usize::try_from(coef.block_count()).unwrap_or(0);
    let blocks = aux.pixel_diff.x.as_chunks_mut::<64>().0;
    for (i, block) in blocks.iter_mut().take(block_count).enumerate() {
        dct8x8(block);
        coef.clamp_dct_block(i, block);
    }

    aux.cos.clone_from(&aux.pixel_diff.x);

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
        coef.rounded_px_w(),
        coef.rounded_px_h(),
        coef.block_w(),
        coef.block_h(),
    );

    if resample {
        upsample_mean(max_w, aux, coef);
    }
}

fn downsample_mean<C: Coef>(max_w: u32, max_h: u32, aux: &mut Aux, coef: &C) {
    let v_samp = coef.vert_factor();
    let h_samp = coef.horiz_factor();
    let samp_area = f32::from(u16::try_from(h_samp.saturating_mul(v_samp)).unwrap_or(1));
    for cy in 0..coef.rounded_px_h() {
        for cx in 0..coef.rounded_px_w() {
            let mut mean = 0.0;
            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);
                    debug_assert!(y < max_h && x < max_w);
                    if let Some(idx) = y
                        .checked_mul(max_w)
                        .and_then(|v| v.checked_add(x))
                        .and_then(|v| usize::try_from(v).ok())
                        && let Some(&val) = aux.fdata.get(idx)
                    {
                        mean += val;
                    }
                }
            }
            mean /= samp_area;
            if let Some(idx) = cy
                .checked_mul(coef.rounded_px_w())
                .and_then(|v| v.checked_add(cx))
                .and_then(|v| usize::try_from(v).ok())
                && let Some(slot) = aux.pixel_diff.y.get_mut(idx)
            {
                *slot = mean;
            }
            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);
                    if let Some(idx) = y
                        .checked_mul(max_w)
                        .and_then(|v| v.checked_add(x))
                        .and_then(|v| usize::try_from(v).ok())
                        && let Some(slot) = aux.fdata.get_mut(idx)
                    {
                        *slot -= mean;
                    }
                }
            }
        }
    }
}

fn upsample_mean<C: Coef>(max_w: u32, aux: &mut Aux, coef: &C) {
    let v_samp = coef.vert_factor();
    let h_samp = coef.horiz_factor();
    for cy in 0..coef.rounded_px_h() {
        for cx in 0..coef.rounded_px_w() {
            let mean = cy
                .checked_mul(coef.rounded_px_w())
                .and_then(|v| v.checked_add(cx))
                .and_then(|v| usize::try_from(v).ok())
                .and_then(|idx| aux.pixel_diff.y.get(idx).copied())
                .unwrap_or(0.0);
            for sy in 0..v_samp {
                for sx in 0..h_samp {
                    let y = cy.saturating_mul(v_samp).saturating_add(sy);
                    let x = cx.saturating_mul(h_samp).saturating_add(sx);
                    if let Some(idx) = y
                        .checked_mul(max_w)
                        .and_then(|v| v.checked_add(x))
                        .and_then(|v| usize::try_from(v).ok())
                        && let Some(slot) = aux.fdata.get_mut(idx)
                    {
                        *slot += mean;
                    }
                }
            }
        }
    }
}
