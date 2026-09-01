use crate::pipeline::simd8::{coef::SIMD8Coef, f32x8};
use crate::utils::traits::{Clamp, FromSlice, WriteTo};
use crate::utils::{auxiliary::Aux, projection};

pub fn compute_projection(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &SIMD8Coef,
) {
    projection::projection(
        max_rounded_px_w,
        max_rounded_px_h,
        aux,
        coef,
        |data, coef| {
            for i in 0..coef.block_count as usize {
                for j in 0..8 {
                    let a = i * 64 + j * 8;
                    let b = a + 7;
                    let old = &mut data[a..=b];
                    let max = coef.dequant_dct_coefs_max[i * 8 + j];
                    let min = coef.dequant_dct_coefs_min[i * 8 + j];
                    f32x8::from_slc(old).clmp(min, max).write_to(old);
                }
            }
        },
    );
}
