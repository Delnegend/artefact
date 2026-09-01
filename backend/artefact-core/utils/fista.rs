use rayon::prelude::*;

use crate::utils::{auxiliary::Aux, coef::Coef, macros::mul_add};

pub fn fista_loop<C, F>(
    mut auxs: Vec<Aux>,
    coefs: &[C],
    iterations: usize,
    max_rounded_px_count: usize,
    radius: f32,
    mut step_fn: F,
) -> Vec<Vec<f32>>
where
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
        step_fn(coefs, &mut auxs, step_size);
    }

    auxs.into_par_iter().map(|aux| aux.fdata).collect()
}

pub fn init_auxs<C>(max_w: u32, max_h: u32, max_count: usize, coefs: &[C]) -> Vec<Aux>
where
    C: Coef,
{
    coefs
        .iter()
        .map(|c| Aux::init(max_w, max_h, max_count, c))
        .collect()
}

pub fn radius(max_count: usize) -> f32 {
    (max_count as f32).sqrt() / 2.0
}
