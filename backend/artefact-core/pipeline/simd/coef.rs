use std::{ops::Mul, simd::f32x64};

use zune_jpeg::sample_factor::SampleFactor;

use crate::{
    jpeg::Coefficient,
    utils::{
        auxiliary::AuxTraits,
        dct::idct8x8s,
        traits::{FromSlice, WriteTo},
    },
};

#[derive(Debug, Clone, Default)]
pub struct SIMDCoef {
    pub rounded_px_w: u32,
    pub rounded_px_h: u32,
    pub rounded_px_count: u32,

    pub block_w: u32,
    pub block_h: u32,
    pub block_count: u32,

    pub horizontal_samp_factor: SampleFactor,
    pub vertical_samp_factor: SampleFactor,

    pub dct_coefs: Vec<f32x64>,
    pub quant_table: f32x64,
    pub quant_table_squared: f32x64,

    pub dequant_dct_coefs_min: Vec<f32x64>,
    pub dequant_dct_coefs_max: Vec<f32x64>,
    pub image_data: Vec<f32>,
}

impl From<Coefficient> for SIMDCoef {
    fn from(c: Coefficient) -> Self {
        let dct_coefs = c
            .dct_coefs
            .as_chunks::<64>()
            .0
            .iter()
            .map(|c| f32x64::from_slc(c))
            .collect::<Vec<f32x64>>();

        let quant_table = f32x64::from_array(c.quant_table);

        Self {
            rounded_px_w: c.rounded_px_w,
            rounded_px_h: c.rounded_px_h,
            rounded_px_count: c.rounded_px_count,
            block_w: c.block_w,
            block_h: c.block_h,
            block_count: c.block_count,
            horizontal_samp_factor: c.horizontal_samp_factor,
            vertical_samp_factor: c.vertical_samp_factor,

            quant_table_squared: quant_table * quant_table,

            dequant_dct_coefs_min: dct_coefs
                .iter()
                .map(|dct_coefs| (*dct_coefs - f32x64::splat(0.5)) * quant_table)
                .collect(),

            dequant_dct_coefs_max: dct_coefs
                .iter()
                .map(|dct_coefs| (*dct_coefs + f32x64::splat(0.5)) * quant_table)
                .collect(),

            image_data: {
                let mut image_data = vec![0.0; c.rounded_px_count as usize];
                let block_w = c.block_w as usize;
                let rounded_px_w = c.rounded_px_w as usize;

                for (i, dct) in dct_coefs.iter().enumerate() {
                    let mut block = [0.0_f32; 64];
                    (*dct * quant_table).write_to(&mut block);

                    idct8x8s(&mut block);

                    // 8x8 -> raster
                    let block_y = i / block_w;
                    let block_x = i % block_w;
                    for in_y in 0..8 {
                        let row = (block_y * 8 + in_y) * rounded_px_w + block_x * 8;
                        for in_x in 0..8 {
                            image_data[row + in_x] = block[in_y * 8 + in_x];
                        }
                    }
                }

                image_data
            },

            dct_coefs,
            quant_table,
        }
    }
}

impl AuxTraits for SIMDCoef {
    fn cos_count(&self) -> usize {
        self.rounded_px_count as usize
    }

    fn get_fdata(&self, max_rounded_px_w: u32, max_rounded_px_h: u32, out: &mut [f32]) {
        crate::utils::auxiliary::upsample_fdata(
            &self.image_data,
            self.rounded_px_w,
            self.rounded_px_h,
            self.horizontal_samp_factor.usize(),
            self.vertical_samp_factor.usize(),
            max_rounded_px_w,
            max_rounded_px_h,
            out,
        );
    }

    fn get_cos(&self, out: &mut [f32]) {
        for i in 0..self.block_count as usize {
            self.dct_coefs[i]
                .mul(self.quant_table)
                .write_to(&mut out[i * 64..(i + 1) * 64]);
        }
    }
}

impl crate::utils::coef::Coef for SIMDCoef {
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
        let max = self.dequant_dct_coefs_max[block_idx];
        let min = self.dequant_dct_coefs_min[block_idx];
        f32x64::from_slice(data).simd_clamp(min, max).write_to(data);
    }
}
