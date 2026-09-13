use rayon::prelude::*;

use super::coef::Coef;
use crate::utils::{auxiliary::Aux, macros::mul_add};

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
            for i in 0..max_rounded_px_count {
                aux.fista[i] = mul_add!(factor, aux.fdata[i] - aux.fista[i], aux.fdata[i]);
            }
            std::mem::swap(&mut aux.fdata, &mut aux.fista);
        });

        term = next_term;

        let step_size = radius / (1.0 + iterations as f32).sqrt();
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
    (max_count as f32).sqrt() / 2.0
}
