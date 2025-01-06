use super::aux::Aux;

/// Computes the Total Variation (TV) regularization term and its gradient
pub fn compute_step_tv(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) -> f64 {
    let mut tv = 0.0;

    for curr_row in 0..max_rounded_px_h {
        for curr_row_idx in 0..max_rounded_px_w {
            compute_step_tv_inner(
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

pub fn compute_step_tv_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_row_idx: u32,
    curr_row: u32,
    tv: &mut f64,
) {
    let mut g_xs = [0.0; 3];
    let mut g_ys = [0.0; 3];

    let curr_px_idx = (curr_row * max_rounded_px_w + curr_row_idx) as usize;
    let next_px_idx = curr_px_idx + 1;
    let below_px_idx = ((curr_row + 1) * max_rounded_px_w + curr_row_idx) as usize;

    let px_at_right_edge = curr_row_idx >= max_rounded_px_w - 1;
    let px_at_bottom_edge = curr_row >= max_rounded_px_h - 1;

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // forward difference x
        if !px_at_right_edge {
            g_xs[c] = aux.fdata[next_px_idx] - aux.fdata[curr_px_idx];
        }

        // forward difference y
        if !px_at_bottom_edge {
            g_ys[c] = aux.fdata[below_px_idx] - aux.fdata[curr_px_idx];
        }
    }

    // norm
    let mut g_norm = 0.0;
    for c in 0..nchannel {
        g_norm += g_xs[c].powi(2);
        g_norm += g_ys[c].powi(2);
    }
    g_norm = g_norm.sqrt();

    let alpha = 1.0 / (nchannel as f32).sqrt();
    *tv += alpha as f64 * g_norm as f64;

    // compute derivatives
    if g_norm != 0.0 {
        for c in 0..nchannel {
            auxs[c].obj_gradient[curr_px_idx] += alpha * -(g_xs[c] + g_ys[c]) / g_norm;

            if !px_at_right_edge {
                auxs[c].obj_gradient[next_px_idx] += alpha * g_xs[c] / g_norm;
            }

            if !px_at_bottom_edge {
                auxs[c].obj_gradient[below_px_idx] += alpha * g_ys[c] / g_norm;
            }
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        auxs[c].pixel_diff.x[curr_px_idx] = g_xs[c];
        auxs[c].pixel_diff.y[curr_px_idx] = g_ys[c];
    }
}
