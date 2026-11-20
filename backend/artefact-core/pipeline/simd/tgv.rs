use std::{
    ops::{Add, Div, Mul, Neg, Sub},
    simd::{Simd, StdFloat, cmp::SimdPartialEq},
};

use super::{
    adaptive_width::{AdaptiveWidth, dispatch_run},
    run::{add_shifted_left, load, load_shifted_right, run_mut},
    traits::{AddSlice, SafeDiv},
};
use crate::utils::auxiliary::Aux;

/// Second-order Total Generalized Variation (TGV) gradient: each run in
/// `adaptive_widths` is processed with the matching `Simd<f32, N>` lane width.
pub fn tgv_gradient(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
    adaptive_widths: &[AdaptiveWidth],
    _norm: &mut [f32],
) {
    let nch_f32 = f32::from(u16::try_from(nchannel).unwrap_or(u16::MAX));
    let alpha = alpha / nch_f32.sqrt();

    for curr_row in 0..max_rounded_px_h {
        for &adaptive_width in adaptive_widths {
            dispatch_run!(
                adaptive_width,
                tgv_run,
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                alpha;
                curr_row
            );
        }
    }
}

/// Second-order Total Generalized Variation (TGV) backward-difference gradient
/// for one run of `N` lanes.
fn tgv_run<const N: usize>(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    alpha: f32,
    curr_row_px_idx: u32,
    curr_row: u32,
) {
    let pad = N.saturating_sub(8);
    let n_minus_1 = N.saturating_sub(1);
    let w_usize = usize::try_from(max_rounded_px_w).unwrap_or(0);
    let idx = usize::try_from(
        curr_row
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_px_idx),
    )
    .unwrap_or(0);
    let at_top = curr_row == 0;
    let at_left = curr_row_px_idx == 0;
    let at_bottom = curr_row == max_rounded_px_h.saturating_sub(1);
    let at_right = usize::try_from(curr_row_px_idx)
        .unwrap_or(usize::MAX)
        .saturating_add(8)
        .saturating_add(pad)
        >= w_usize;
    let above = usize::try_from(
        curr_row
            .saturating_sub(1)
            .saturating_mul(max_rounded_px_w)
            .saturating_add(curr_row_px_idx),
    )
    .unwrap_or(0);

    let grads = compute_second_order(auxs, nchannel, idx, above, n_minus_1, at_top, at_left);

    let two = Simd::<f32, N>::splat(2.0);
    let g2_norm = grads
        .xx
        .iter()
        .zip(grads.yy.iter())
        .zip(grads.xy_sym.iter())
        .take(nchannel)
        .map(|((&g_xx, &g_yy), &g_xy_sym)| {
            g_xx.mul(g_xx)
                .add(two.mul(g_xy_sym).mul(g_xy_sym))
                .add(g_yy.mul(g_yy))
        })
        .fold(Simd::<f32, N>::splat(0.0), Add::add)
        .sqrt();

    let state = TgvRunState {
        nchannel,
        g_xxs: &grads.xx,
        g_yys: &grads.yy,
        g_xy_syms: &grads.xy_sym,
        g2_norm,
        alpha: Simd::<f32, N>::splat(alpha),
        idx,
        above,
        w_usize,
        n_minus_1,
        horizontal_edges: (at_left, at_right),
        vertical_edges: (at_top, at_bottom),
    };
    scatter_tgv(auxs, &state);
}

struct SecondOrderGrads<const N: usize> {
    xx: [Simd<f32, N>; 3],
    yy: [Simd<f32, N>; 3],
    xy_sym: [Simd<f32, N>; 3],
}

fn compute_second_order<const N: usize>(
    auxs: &[Aux],
    nchannel: usize,
    idx: usize,
    above: usize,
    n_minus_1: usize,
    at_top: bool,
    at_left: bool,
) -> SecondOrderGrads<N> {
    let mut g_xxs = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_yys = [Simd::<f32, N>::splat(0.0); 3];
    let mut g_xy_syms = [Simd::<f32, N>::splat(0.0); 3];
    for (((aux, g_xx), g_yy), g_xy_sym) in auxs
        .iter()
        .zip(g_xxs.iter_mut())
        .zip(g_yys.iter_mut())
        .zip(g_xy_syms.iter_mut())
        .take(nchannel)
    {
        let g_yx = if at_left {
            load_shifted_right::<N>(&aux.pixel_diff.y, idx.saturating_add(1), n_minus_1)
                .sub(load_shifted_right::<N>(&aux.pixel_diff.y, idx, n_minus_1))
        } else {
            load::<N>(&aux.pixel_diff.y, idx)
                .sub(load::<N>(&aux.pixel_diff.y, idx.saturating_sub(1)))
        };

        let g_xy = if at_top {
            Simd::<f32, N>::splat(0.0)
        } else {
            load::<N>(&aux.pixel_diff.x, idx).sub(load::<N>(&aux.pixel_diff.x, above))
        };

        *g_xx = if at_left {
            load_shifted_right::<N>(&aux.pixel_diff.x, idx.saturating_add(1), n_minus_1)
                .sub(load_shifted_right::<N>(&aux.pixel_diff.x, idx, n_minus_1))
        } else {
            load::<N>(&aux.pixel_diff.x, idx)
                .sub(load::<N>(&aux.pixel_diff.x, idx.saturating_sub(1)))
        };

        *g_yy = if at_top {
            Simd::<f32, N>::splat(0.0)
        } else {
            load::<N>(&aux.pixel_diff.y, idx).sub(load::<N>(&aux.pixel_diff.y, above))
        };

        *g_xy_sym = g_xy.add(g_yx).div(Simd::<f32, N>::splat(2.0));
    }

    SecondOrderGrads {
        xx: g_xxs,
        yy: g_yys,
        xy_sym: g_xy_syms,
    }
}


struct TgvRunState<'a, const N: usize> {
    nchannel: usize,
    g_xxs: &'a [Simd<f32, N>; 3],
    g_yys: &'a [Simd<f32, N>; 3],
    g_xy_syms: &'a [Simd<f32, N>; 3],
    g2_norm: Simd<f32, N>,
    alpha: Simd<f32, N>,
    idx: usize,
    above: usize,
    w_usize: usize,
    n_minus_1: usize,
    horizontal_edges: (bool, bool),
    vertical_edges: (bool, bool),
}

fn scatter_tgv<const N: usize>(auxs: &mut [Aux], s: &TgvRunState<'_, N>) {
    let two = Simd::<f32, N>::splat(2.0);
    let mask = s.g2_norm.simd_ne(Simd::<f32, N>::splat(0.0));

    for (((aux, &g_xx), &g_yy), &g_xy_sym) in auxs
        .iter_mut()
        .zip(s.g_xxs.iter())
        .zip(s.g_yys.iter())
        .zip(s.g_xy_syms.iter())
        .take(s.nchannel)
    {
        let target = run_mut(&mut aux.obj_gradient, s.idx, N);
        s.alpha
            .mul(
                two.mul(g_xx)
                    .add(two.mul(g_xy_sym))
                    .add(two.mul(g_yy))
                    .neg(),
            )
            .div(s.g2_norm)
            .add_slice(target)
            .store_select(target, mask);

        scatter_horizontal(&mut aux.obj_gradient, s, g_xx, g_xy_sym, mask);
        scatter_vertical_and_diag(&mut aux.obj_gradient, s, g_yy, g_xy_sym, mask);
    }
}

fn scatter_horizontal<const N: usize>(
    obj_gradient: &mut [f32],
    s: &TgvRunState<'_, N>,
    g_xx: Simd<f32, N>,
    g_xy_sym: Simd<f32, N>,
    mask: std::simd::Mask<i32, N>,
) {
    let gx_term = s.alpha.mul(g_xy_sym.add(g_xx));
    if s.horizontal_edges.0 {
        let target = run_mut(obj_gradient, s.idx, s.n_minus_1);
        add_shifted_left::<N>(target, gx_term.safe_div(s.g2_norm));
    } else {
        let target = run_mut(obj_gradient, s.idx.saturating_sub(1), N);
        gx_term
            .div(s.g2_norm)
            .add_slice(target)
            .store_select(target, mask);
    }

    if s.horizontal_edges.1 {
        let target = run_mut(obj_gradient, s.idx.saturating_add(1), s.n_minus_1);
        gx_term
            .div(s.g2_norm)
            .add_short_slice(target)
            .store_select(target, mask);
    } else {
        let target = run_mut(obj_gradient, s.idx.saturating_add(1), N);
        gx_term
            .div(s.g2_norm)
            .add_slice(target)
            .store_select(target, mask);
    }
}

fn scatter_vertical_and_diag<const N: usize>(
    obj_gradient: &mut [f32],
    s: &TgvRunState<'_, N>,
    g_yy: Simd<f32, N>,
    g_xy_sym: Simd<f32, N>,
    mask: std::simd::Mask<i32, N>,
) {
    let below = s.idx.saturating_add(s.w_usize);
    let vertical_term = s.alpha.mul(g_yy.add(g_xy_sym));
    if !s.vertical_edges.0 {
        let target = run_mut(obj_gradient, s.above, N);
        vertical_term
            .div(s.g2_norm)
            .add_slice(target)
            .store_select(target, mask);
    }
    if !s.vertical_edges.1 {
        let target = run_mut(obj_gradient, below, N);
        vertical_term
            .div(s.g2_norm)
            .add_slice(target)
            .store_select(target, mask);
    }

    let diag_term = s.alpha.mul(g_xy_sym.neg());
    if !s.vertical_edges.0 {
        let base = s.idx.saturating_add(1).saturating_sub(s.w_usize);
        if s.horizontal_edges.1 {
            let target = run_mut(obj_gradient, base, s.n_minus_1);
            diag_term
                .div(s.g2_norm)
                .add_short_slice(target)
                .store_select(target, mask);
        } else {
            let target = run_mut(obj_gradient, base, N);
            diag_term
                .div(s.g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }
    }
    if !s.vertical_edges.1 {
        if s.horizontal_edges.0 {
            let target = run_mut(obj_gradient, below, s.n_minus_1);
            add_shifted_left::<N>(target, diag_term.safe_div(s.g2_norm));
        } else {
            let target = run_mut(obj_gradient, below.saturating_sub(1), N);
            diag_term
                .div(s.g2_norm)
                .add_slice(target)
                .store_select(target, mask);
        }
    }
}
