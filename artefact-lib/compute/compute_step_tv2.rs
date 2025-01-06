use crate::compute::aux::Aux;

/// Computes the Total Generalized Variation (TGV) regularization term and its gradient
#[allow(unused)]
pub fn compute_step_tv2(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
) -> f64 {
    let mut tv2 = 0.0;

    for y in 0..max_rounded_px_h {
        for x in 0..max_rounded_px_w {
            compute_step_tv2_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                alpha,
                x,
                y,
                &mut tv2,
            );
        }
    }

    tv2
}

#[allow(clippy::too_many_arguments)]
fn compute_step_tv2_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
    curr_x: u32,
    curr_y: u32,
    tv2: &mut f64,
) {
    let mut g_xxs = [0.0; 3];
    let mut g_xy_syms = [0.0; 3];
    let mut g_yys = [0.0; 3];

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // backward difference x
        g_xxs[c] = match curr_x {
            ..=0 => 0.0,
            _ => {
                let curr_idx = (curr_y * max_rounded_px_w + curr_x) as usize;
                let prev_idx = (curr_y * max_rounded_px_w + (curr_x - 1)) as usize;

                aux.pixel_diff.x[curr_idx] - aux.pixel_diff.x[prev_idx]
            }
        };

        // backward difference x
        let g_yx = match curr_x {
            ..=0 => 0.0,
            _ => {
                let curr_idx = (curr_y * max_rounded_px_w + curr_x) as usize;
                let prev_idx = (curr_y * max_rounded_px_w + (curr_x - 1)) as usize;

                aux.pixel_diff.y[curr_idx] - aux.pixel_diff.y[prev_idx]
            }
        };

        // backward difference y
        let g_xy = match curr_y {
            ..=0 => 0.0,
            _ => {
                let curr_idx = (curr_y * max_rounded_px_w + curr_x) as usize;
                let prev_idx = ((curr_y - 1) * max_rounded_px_w + curr_x) as usize;

                aux.pixel_diff.x[curr_idx] - aux.pixel_diff.x[prev_idx]
            }
        };

        // backward difference y
        g_yys[c] = match curr_y {
            ..=0 => 0.0,
            _ => {
                let curr_idx = (curr_y * max_rounded_px_w + curr_x) as usize;
                let prev_idx = ((curr_y - 1) * max_rounded_px_w + curr_x) as usize;

                aux.pixel_diff.y[curr_idx] - aux.pixel_diff.y[prev_idx]
            }
        };

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / 2.0;
    }

    // norm
    let g2_norm = {
        let mut g2_norm = 0.0;
        for c in 0..nchannel {
            g2_norm += g_xxs[c].powi(2) + 2.0 * g_xy_syms[c].powi(2) + g_yys[c].powi(2);
        }
        g2_norm.sqrt()
    };

    let mut alpha = alpha;
    alpha /= (nchannel as f32).sqrt();
    *tv2 += (alpha * g2_norm) as f64; // objective function

    if g2_norm == 0.0 {
        return;
    }

    // compute derivatives
    for c in 0..nchannel {
        let g_xx = g_xxs[c];
        let g_yy = g_yys[c];
        let g_xy_sym = g_xy_syms[c];
        let aux = &mut auxs[c];

        aux.obj_gradient[(curr_y * max_rounded_px_w + curr_x) as usize] +=
            alpha * (-(2.0 * g_xx + 2.0 * g_xy_sym + 2.0 * g_yy) / g2_norm);

        if curr_x > 0 {
            aux.obj_gradient[(curr_y * max_rounded_px_w + (curr_x - 1)) as usize] +=
                alpha * ((g_xy_sym + g_xx) / g2_norm);
        }

        if curr_x < max_rounded_px_w - 1 {
            aux.obj_gradient[(curr_y * max_rounded_px_w + (curr_x + 1)) as usize] +=
                alpha * ((g_xy_sym + g_xx) / g2_norm);
        }

        if curr_y > 0 {
            aux.obj_gradient[((curr_y - 1) * max_rounded_px_w + curr_x) as usize] +=
                alpha * ((g_yy + g_xy_sym) / g2_norm);
        }

        if curr_y < max_rounded_px_h - 1 {
            aux.obj_gradient[((curr_y + 1) * max_rounded_px_w + curr_x) as usize] +=
                alpha * ((g_yy + g_xy_sym) / g2_norm);
        }

        if curr_x < max_rounded_px_w - 1 && curr_y > 0 {
            aux.obj_gradient[((curr_y - 1) * max_rounded_px_w + (curr_x + 1)) as usize] +=
                alpha * ((-g_xy_sym) / g2_norm);
        }

        if curr_x > 0 && curr_y < max_rounded_px_h - 1 {
            aux.obj_gradient[((curr_y + 1) * max_rounded_px_w + (curr_x - 1)) as usize] +=
                alpha * ((-g_xy_sym) / g2_norm);
        }
    }
}
