use crate::{compute::aux::Aux, utils::macros::mul_add};

/// Computes the Total Generalized Variation (TGV) regularization term and its gradient
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
        #[allow(clippy::useless_let_if_seq)]
        let mut g_xy = 0.0;
        #[allow(clippy::useless_let_if_seq)]
        let mut g_yx = 0.0;

        // backward difference x
        if curr_x != 0 {
            let a = (curr_y * max_rounded_px_w + curr_x) as usize;
            let b = (curr_y * max_rounded_px_w + (curr_x - 1)) as usize;

            g_xxs[c] = aux.pixel_diff.x[a] - aux.pixel_diff.x[b];
            g_yx = aux.pixel_diff.y[a] - aux.pixel_diff.y[b];
        }

        // backward difference y
        if curr_y != 0 {
            let a = (curr_y * max_rounded_px_w + curr_x) as usize;
            let b = ((curr_y - 1) * max_rounded_px_w + curr_x) as usize;

            g_yys[c] = aux.pixel_diff.y[a] - aux.pixel_diff.y[b];
            g_xy = aux.pixel_diff.x[a] - aux.pixel_diff.x[b];
        }

        // symmetrize
        g_xy_syms[c] = (g_xy + g_yx) / 2.0;
    }

    // norm
    let g2_norm = {
        let mut g2_norm = 0.0;
        for c in 0..nchannel {
            g2_norm += mul_add!(
                g_yys[c],
                g_yys[c],
                mul_add!(2.0_f32, g_xy_syms[c].powi(2), g_xxs[c].powi(2))
            );
        }
        g2_norm.sqrt()
    };

    let mut alpha = alpha;
    alpha /= (nchannel as f32).sqrt();
    *tv2 += f64::from(alpha * g2_norm); // objective function

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
            alpha * (-(mul_add!(2.0_f32, g_yy, mul_add!(2.0_f32, g_xx, 2.0 * g_xy_sym))) / g2_norm);

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
