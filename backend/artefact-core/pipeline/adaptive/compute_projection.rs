use std::simd::{f32x64, num::SimdFloat};

use crate::pipeline::adaptive::coef::SIMDAdaptiveCoef;
use crate::utils::{auxiliary::Aux, projection, traits::WriteTo};

pub fn compute_projection(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &SIMDAdaptiveCoef,
) {
    projection::projection(
        max_rounded_px_w,
        max_rounded_px_h,
        aux,
        coef,
        |data, coef| {
            for i in 0..coef.block_count as usize {
                let a = i * 64;
                let b = a + 63;
                let old = &mut data[a..=b];
                let max = coef.dequant_dct_coefs_max[i];
                let min = coef.dequant_dct_coefs_min[i];
                f32x64::from_slice(old).simd_clamp(min, max).write_to(old);
            }
        },
    );
}
