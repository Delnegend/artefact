use std::{ops::Mul, simd::f32x8};

use crate::{
    jpeg::Coefficient,
    utils::{
        auxiliary::AuxTraits,
        boxing::unboxing,
        dct::idct8x8s,
        traits::{FromSlice, WriteTo},
    },
};
use zune_jpeg::sample_factor::SampleFactor;

#[derive(Debug, Clone, Default)]
pub struct SIMD8Coef {
    pub rounded_px_w: u32,
    pub rounded_px_h: u32,
    pub rounded_px_count: u32,

    pub block_w: u32,
    pub block_h: u32,
    pub block_count: u32,

    pub horizontal_samp_factor: SampleFactor,
    pub vertical_samp_factor: SampleFactor,

    pub dct_coefs: Vec<f32x8>,
    pub quant_table: [f32x8; 8],
    pub quant_table_squared: [f32x8; 8],

    pub dequant_dct_coefs_min: Vec<f32x8>,
    pub dequant_dct_coefs_max: Vec<f32x8>,
    pub image_data: Vec<f32>,
}

impl From<Coefficient> for SIMD8Coef {
    fn from(c: Coefficient) -> Self {
        let dct_coefs = c
            .dct_coefs
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| f32x8::from_slc(c))
            .collect::<Vec<f32x8>>();

        let quant_table: [f32x8; 8] = c
            .quant_table
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| f32x8::from_slc(c))
            .collect::<Vec<f32x8>>()
            .try_into()
            .expect("Invalid quant_table length");

        Self {
            rounded_px_w: c.rounded_px_w,
            rounded_px_h: c.rounded_px_h,
            rounded_px_count: c.rounded_px_count,
            block_w: c.block_w,
            block_h: c.block_h,
            block_count: c.block_count,
            horizontal_samp_factor: c.horizontal_samp_factor,
            vertical_samp_factor: c.vertical_samp_factor,

            quant_table_squared: quant_table
                .iter()
                .map(|&x| x * x)
                .collect::<Vec<f32x8>>()
                .try_into()
                .expect("Invalid quant_table_squared length"),

            dequant_dct_coefs_min: dct_coefs
                .iter()
                .enumerate()
                .map(|(idx, dct_coefs)| {
                    let quant_table = quant_table[idx % 8];
                    (*dct_coefs - f32x8::splat(0.5)) * quant_table
                })
                .collect(),

            dequant_dct_coefs_max: dct_coefs
                .iter()
                .enumerate()
                .map(|(idx, dct_coefs)| {
                    let quant_table = quant_table[idx % 8];
                    (*dct_coefs + f32x8::splat(0.5)) * quant_table
                })
                .collect(),

            image_data: {
                let mut tmp = vec![0.0; c.rounded_px_count as usize];

                for i in 0..(c.block_count as usize) {
                    for j in 0..8 {
                        let result = dct_coefs[i * 8 + j] * quant_table[j];

                        let idx = i * 64 + j * 8;
                        result.write_to(&mut tmp[idx..idx + 8]);
                    }

                    idct8x8s(
                        tmp[i * 64..(i + 1) * 64]
                            .as_mut()
                            .try_into()
                            .expect("Invalid image_data length"),
                    );
                }

                // 8x8 -> 64x1
                unboxing(
                    &tmp.clone(),
                    tmp.as_mut(),
                    c.rounded_px_w,
                    c.rounded_px_h,
                    c.block_w,
                    c.block_h,
                );

                tmp
            },

            dct_coefs,
            quant_table,
        }
    }
}

impl AuxTraits for SIMD8Coef {
    fn get_fdata(
        &self,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
    ) -> Vec<f32> {
        crate::utils::auxiliary::upsample_fdata(
            &self.image_data,
            self.rounded_px_w,
            self.rounded_px_h,
            self.horizontal_samp_factor.usize(),
            self.vertical_samp_factor.usize(),
            max_rounded_px_w,
            max_rounded_px_h,
            max_rounded_px_count,
        )
    }

    fn get_cos(&self) -> Vec<f32> {
        let mut cos = vec![0.0; (self.rounded_px_count) as usize];
        for i in 0..self.block_count as usize {
            for j in 0..8 {
                let a = i * 8 + j;
                let b = (i + 1) * 8 + j;

                self.dct_coefs[a]
                    .mul(self.quant_table[j])
                    .write_to(&mut cos[a..b]);
            }
        }
        cos
    }
}

impl crate::utils::coef::Coef for SIMD8Coef {
    fn rounded_px_w(&self) -> u32 {
        self.rounded_px_w
    }
    fn rounded_px_h(&self) -> u32 {
        self.rounded_px_h
    }
    fn block_w(&self) -> u32 {
        self.block_w
    }
    fn block_h(&self) -> u32 {
        self.block_h
    }
    fn block_count(&self) -> u32 {
        self.block_count
    }
    fn horiz_factor(&self) -> u32 {
        self.horizontal_samp_factor.u32()
    }
    fn vert_factor(&self) -> u32 {
        self.vertical_samp_factor.u32()
    }
    fn clamp_block(&self, block_idx: usize, data: &mut [f32]) {
        use crate::utils::traits::WriteTo;
        use std::simd::num::SimdFloat;
        for j in 0..8 {
            let a = j * 8;
            let b = a + 7;
            let old = &mut data[a..=b];
            let max = self.dequant_dct_coefs_max[block_idx * 8 + j];
            let min = self.dequant_dct_coefs_min[block_idx * 8 + j];
            f32x8::from_slice(old).simd_clamp(min, max).write_to(old);
        }
    }
}
