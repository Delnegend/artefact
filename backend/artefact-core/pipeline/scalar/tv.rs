use crate::utils::auxiliary::Aux;

/// Computes the Total Variation (TV) regularization term and its gradient
pub fn tv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) -> f64 {
    let mut tv = 0.0;

    for curr_row in 0..max_rounded_px_h {
        for curr_row_idx in 0..max_rounded_px_w {
            tv_gradient_px(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                curr_row_idx,
                curr_row,
                &mut tv,
            );
        }
    }

    tv
}

pub fn tv_gradient_px(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_idx: u32,
    curr_row: u32,
    tv: &mut f64,
) {
    let mut g_xs = [0.0; 3];
    let mut g_verts = [0.0; 3];

    let Ok(curr_px_idx) = usize::try_from(
        curr_row
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_idx),
    ) else {
        return;
    };
    let next_px_idx = curr_px_idx.saturating_add(1);
    let Ok(below_px_idx) = usize::try_from(
        curr_row
            .saturating_add(1)
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_idx),
    ) else {
        return;
    };

    let px_at_right_edge = curr_row_idx >= max_rounded_px_w.saturating_sub(1);
    let px_at_bottom_edge = curr_row >= max_rounded_px_h.saturating_sub(1);

    for ((aux, gx), gv) in auxs
        .iter_mut()
        .zip(g_xs.iter_mut())
        .zip(g_verts.iter_mut())
        .take(nchannel)
    {
        let curr_val = aux.fdata.get(curr_px_idx).copied().unwrap_or(0.0);
        // forward difference x
        if !px_at_right_edge && let Some(&next_val) = aux.fdata.get(next_px_idx) {
            *gx = next_val - curr_val;
        }

        // forward difference y
        if !px_at_bottom_edge && let Some(&below_val) = aux.fdata.get(below_px_idx) {
            *gv = below_val - curr_val;
        }
    }

    // norm
    let mut g_norm = 0.0_f32;
    for (&gx, &gv) in g_xs.iter().zip(g_verts.iter()).take(nchannel) {
        g_norm += gx.mul_add(gx, gv.powi(2));
    }
    g_norm = g_norm.sqrt();

    let nch_f32 = f32::from(u16::try_from(nchannel).unwrap_or(u16::MAX));
    let alpha = 1.0 / nch_f32.sqrt();
    *tv = f64::mul_add(f64::from(alpha), f64::from(g_norm), *tv);

    // compute derivatives
    if g_norm != 0.0 {
        for ((aux, &gx), &gv) in auxs
            .iter_mut()
            .zip(g_xs.iter())
            .zip(g_verts.iter())
            .take(nchannel)
        {
            if let Some(slot) = aux.obj_gradient.get_mut(curr_px_idx) {
                *slot = alpha.mul_add(-(gx + gv) / g_norm, *slot);
            }

            if !px_at_right_edge && let Some(slot) = aux.obj_gradient.get_mut(next_px_idx) {
                *slot = alpha.mul_add(gx / g_norm, *slot);
            }

            if !px_at_bottom_edge && let Some(slot) = aux.obj_gradient.get_mut(below_px_idx) {
                *slot = alpha.mul_add(gv / g_norm, *slot);
            }
        }
    }

    // store for the second-order TGV pass
    for ((aux, &gx), &gv) in auxs
        .iter_mut()
        .zip(g_xs.iter())
        .zip(g_verts.iter())
        .take(nchannel)
    {
        if let Some(slot) = aux.pixel_diff.x.get_mut(curr_px_idx) {
            *slot = gx;
        }
        if let Some(slot) = aux.pixel_diff.y.get_mut(curr_px_idx) {
            *slot = gv;
        }
    }
}
