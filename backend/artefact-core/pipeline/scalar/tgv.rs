use crate::utils::{auxiliary::Aux, macros::mul_add};

/// Computes the Total Generalized Variation (TGV) regularization term and its gradient
pub fn tgv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
) -> f64 {
    let mut tv2 = 0.0;

    for y in 0..max_rounded_px_h {
        for x in 0..max_rounded_px_w {
            tgv_gradient_px(
                &TgvPxParams {
                    max_rounded_px_w,
                    max_rounded_px_h,
                    nchannel,
                    alpha,
                    curr_x: x,
                    curr_y: y,
                },
                auxs,
                &mut tv2,
            );
        }
    }

    tv2
}

struct TgvPxParams {
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    alpha: f32,
    curr_x: u32,
    curr_y: u32,
}

fn tgv_gradient_px(params: &TgvPxParams, auxs: &mut [Aux], tv2: &mut f64) {
    let mut g_xxs = [0.0; 3];
    let mut g_xy_syms = [0.0; 3];
    let mut g_yys = [0.0; 3];
    compute_second_order(params, auxs, &mut g_xxs, &mut g_xy_syms, &mut g_yys);

    let mut g2_norm = 0.0_f32;
    for ((&g_xx, &g_xy_sym), &g_yy) in g_xxs
        .iter()
        .zip(g_xy_syms.iter())
        .zip(g_yys.iter())
        .take(params.nchannel)
    {
        g2_norm += mul_add!(
            g_yy,
            g_yy,
            mul_add!(2.0_f32, g_xy_sym.powi(2), g_xx.powi(2))
        );
    }
    let g2_norm = g2_norm.sqrt();

    let nch_f32 = f32::from(u16::try_from(params.nchannel).unwrap_or(u16::MAX));
    let alpha = params.alpha / nch_f32.sqrt();
    *tv2 += f64::from(alpha * g2_norm);
    if g2_norm != 0.0 {
        scatter_tgv_grad(params, auxs, &g_xxs, &g_xy_syms, &g_yys, alpha / g2_norm);
    }
}

fn compute_second_order(
    params: &TgvPxParams,
    auxs: &mut [Aux],
    g_xxs: &mut [f32; 3],
    g_xy_syms: &mut [f32; 3],
    g_yys: &mut [f32; 3],
) {
    let max_rounded_px_w = params.max_rounded_px_w;
    let nchannel = params.nchannel;
    let curr_x = params.curr_x;
    let curr_y = params.curr_y;

    let px_idx = |x: u32, y: u32| -> Option<usize> {
        usize::try_from(y.saturating_mul(max_rounded_px_w).saturating_add(x)).ok()
    };

    for (((aux, g_xx), g_xy_sym), g_yy) in auxs
        .iter_mut()
        .zip(g_xxs.iter_mut())
        .zip(g_xy_syms.iter_mut())
        .zip(g_yys.iter_mut())
        .take(nchannel)
    {
        // backward difference x
        let g_yx = if curr_x != 0
            && let Some(a) = px_idx(curr_x, curr_y)
            && let Some(b) = px_idx(curr_x.saturating_sub(1), curr_y)
        {
            let ax = aux.pixel_diff.x.get(a).copied().unwrap_or(0.0);
            let bx = aux.pixel_diff.x.get(b).copied().unwrap_or(0.0);
            let ay = aux.pixel_diff.y.get(a).copied().unwrap_or(0.0);
            let by = aux.pixel_diff.y.get(b).copied().unwrap_or(0.0);
            *g_xx = ax - bx;
            ay - by
        } else {
            0.0
        };

        // backward difference y
        let g_xy = if curr_y != 0
            && let Some(a) = px_idx(curr_x, curr_y)
            && let Some(b) = px_idx(curr_x, curr_y.saturating_sub(1))
        {
            let ay = aux.pixel_diff.y.get(a).copied().unwrap_or(0.0);
            let by = aux.pixel_diff.y.get(b).copied().unwrap_or(0.0);
            let ax = aux.pixel_diff.x.get(a).copied().unwrap_or(0.0);
            let bx = aux.pixel_diff.x.get(b).copied().unwrap_or(0.0);
            *g_yy = ay - by;
            ax - bx
        } else {
            0.0
        };

        // symmetrize
        *g_xy_sym = f32::midpoint(g_xy, g_yx);
    }
}

fn scatter_tgv_grad(
    params: &TgvPxParams,
    auxs: &mut [Aux],
    g_xxs: &[f32; 3],
    g_xy_syms: &[f32; 3],
    g_yys: &[f32; 3],
    scale: f32,
) {
    let max_rounded_px_w = params.max_rounded_px_w;
    let max_rounded_px_h = params.max_rounded_px_h;
    let curr_x = params.curr_x;
    let curr_y = params.curr_y;
    let px_idx = |x: u32, y: u32| -> Option<usize> {
        usize::try_from(y.saturating_mul(max_rounded_px_w).saturating_add(x)).ok()
    };
    let add_grad = |grad: &mut [f32], x: u32, y: u32, delta: f32| {
        if let Some(idx) = px_idx(x, y)
            && let Some(slot) = grad.get_mut(idx)
        {
            *slot = scale.mul_add(delta, *slot);
        }
    };
    let w_end = max_rounded_px_w.saturating_sub(1);
    let h_end = max_rounded_px_h.saturating_sub(1);
    for (((aux, &g_xx), &g_xy_sym), &g_yy) in auxs
        .iter_mut()
        .zip(g_xxs.iter())
        .zip(g_xy_syms.iter())
        .zip(g_yys.iter())
        .take(params.nchannel)
    {
        let grad = &mut aux.obj_gradient;
        add_grad(
            grad,
            curr_x,
            curr_y,
            -(mul_add!(2.0_f32, g_yy, mul_add!(2.0_f32, g_xx, 2.0 * g_xy_sym))),
        );

        if curr_x > 0 {
            add_grad(grad, curr_x.saturating_sub(1), curr_y, g_xy_sym + g_xx);
        }
        if curr_x < w_end {
            add_grad(grad, curr_x.saturating_add(1), curr_y, g_xy_sym + g_xx);
        }
        if curr_y > 0 {
            add_grad(grad, curr_x, curr_y.saturating_sub(1), g_yy + g_xy_sym);
        }
        if curr_y < h_end {
            add_grad(grad, curr_x, curr_y.saturating_add(1), g_yy + g_xy_sym);
        }
        if curr_x < w_end && curr_y > 0 {
            add_grad(
                grad,
                curr_x.saturating_add(1),
                curr_y.saturating_sub(1),
                -g_xy_sym,
            );
        }
        if curr_x > 0 && curr_y < h_end {
            add_grad(
                grad,
                curr_x.saturating_sub(1),
                curr_y.saturating_add(1),
                -g_xy_sym,
            );
        }
    }
}
