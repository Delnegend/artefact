use std::ops::Mul;

use crate::{
    jpeg::Coefficient,
    pipeline_simd_8::f32x8,
    utils::{
        aux::AuxTraits,
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
            .chunks_exact(8)
            .map(f32x8::from_slc)
            .collect::<Vec<f32x8>>();

        let quant_table: [f32x8; 8] = c
            .quant_table
            .chunks_exact(8)
            .map(f32x8::from_slc)
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
        let mut fdata = vec![0.0; max_rounded_px_count];

        for y in 0..max_rounded_px_h as usize {
            for x in 0..max_rounded_px_w as usize {
                let cy =
                    (y / self.vertical_samp_factor.usize()).min(self.rounded_px_h as usize - 1);
                let cx =
                    (x / self.horizontal_samp_factor.usize()).min(self.rounded_px_w as usize - 1);

                let fdata_idx = y * max_rounded_px_w as usize + x;
                let img_data_idx = cy * self.rounded_px_w as usize + cx;

                fdata[fdata_idx] = self.image_data[img_data_idx];
            }
        }

        fdata
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
