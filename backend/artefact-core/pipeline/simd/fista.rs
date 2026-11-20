use rayon::prelude::*;

use super::coef::Coef;
use crate::utils::{auxiliary::Aux, macros::mul_add};

fn usize_to_f32(n: usize) -> f32 {
    let clamped = u32::try_from(n).unwrap_or(u32::MAX);
    let hi = f32::from(u16::try_from(clamped >> 16).unwrap_or(u16::MAX));
    let lo = f32::from(u16::try_from(clamped & 0xFFFF).unwrap_or(0));
    hi.mul_add(65536.0, lo)
}

/// FISTA (Fast Iterative Shrinkage-Thresholding Algorithm) accelerated loop:
/// runs `iterations` of the projected subgradient step with Nesterov momentum.
pub fn run_fista<C, F>(
    auxs: &mut [Aux],
    coefs: &[C],
    iterations: usize,
    max_rounded_px_count: usize,
    radius: f32,
    mut step_fn: F,
) where
    C: Coef + Sync,
    F: FnMut(&[C], &mut [Aux], f32),
{
    let mut term = 1.0_f32;

    for _ in 0..iterations {
        let next_term = f32::midpoint(1.0, mul_add!(4.0_f32, term.powi(2), 1.0).sqrt());
        let factor = (term - 1.0) / next_term;

        auxs.par_iter_mut().for_each(|aux| {
            for (fista, &fdata) in aux
                .fista
                .iter_mut()
                .zip(aux.fdata.iter())
                .take(max_rounded_px_count)
            {
                *fista = mul_add!(factor, fdata - *fista, fdata);
            }
            std::mem::swap(&mut aux.fdata, &mut aux.fista);
        });

        term = next_term;

        let step_size = radius / (1.0 + usize_to_f32(iterations)).sqrt();
        step_fn(coefs, auxs, step_size);
    }
}

/// Build the per-channel working state from each component's coefficients.
pub fn init_channel_states<C>(max_w: u32, max_h: u32, max_count: usize, coefs: &[C]) -> Vec<Aux>
where
    C: Coef,
{
    coefs
        .iter()
        .map(|c| Aux::init(max_w, max_h, max_count, c))
        .collect()
}

/// Radius of the feasible box `[-0.5, 0.5]^n`, where `n` is the number of
/// pixel-domain samples.
pub fn box_radius(max_count: usize) -> f32 {
    usize_to_f32(max_count).sqrt() / 2.0
}
