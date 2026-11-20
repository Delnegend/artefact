use std::{
    ops::{Add, Mul, Sub},
    simd::f32x64,
};

use zune_jpeg::sample_factor::SampleFactor;

use super::traits::WriteTo;
use crate::{
    jpeg::Coefficient,
    utils::{auxiliary::AuxTraits, dct::idct8x8},
};

pub trait Coef: AuxTraits {
    fn rounded_px_w(&self) -> u32;
    fn rounded_px_h(&self) -> u32;
    fn block_w(&self) -> u32;
    fn block_h(&self) -> u32;
    fn block_count(&self) -> u32;
    fn horiz_factor(&self) -> u32;
    fn vert_factor(&self) -> u32;

    /// Clamp one dequantized 8x8 Discrete Cosine Transform (DCT) block (64
    /// values) into its quantized interval.
    fn clamp_dct_block(&self, block_idx: usize, data: &mut [f32]);
}

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
            .map(|c| f32x64::from_slice(c))
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

            quant_table_squared: quant_table.mul(quant_table),

            dequant_dct_coefs_min: dct_coefs
                .iter()
                .map(|dct_coefs| dct_coefs.sub(f32x64::splat(0.5)).mul(quant_table))
                .collect(),

            dequant_dct_coefs_max: dct_coefs
                .iter()
                .map(|dct_coefs| dct_coefs.add(f32x64::splat(0.5)).mul(quant_table))
                .collect(),

            image_data: {
                let px_count = usize::try_from(c.rounded_px_count).unwrap_or(0);
                let mut image_data = vec![0.0; px_count];
                let block_w = usize::try_from(c.block_w).unwrap_or(0);
                let rounded_px_w = usize::try_from(c.rounded_px_w).unwrap_or(0);

                if block_w > 0 {
                    for (block_y, row_coefs) in dct_coefs.chunks_exact(block_w).enumerate() {
                        for (block_x, dct) in row_coefs.iter().enumerate() {
                            let mut block = [0.0_f32; 64];
                            dct.mul(quant_table).write_to(&mut block);

                            idct8x8(&mut block);

                            // 8x8 -> raster
                            for (in_y, block_row) in block.as_chunks::<8>().0.iter().enumerate() {
                                let row = block_y
                                    .saturating_mul(8)
                                    .saturating_add(in_y)
                                    .saturating_mul(rounded_px_w)
                                    .saturating_add(block_x.saturating_mul(8));
                                if let Some(dst) = image_data.get_mut(row..row.saturating_add(8)) {
                                    dst.copy_from_slice(block_row);
                                }
                            }
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
        usize::try_from(self.rounded_px_count).unwrap_or(0)
    }

    fn init_pixels_into(&self, max_rounded_px_w: u32, max_rounded_px_h: u32, out: &mut [f32]) {
        crate::utils::auxiliary::upsample_pixels(
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

    fn init_dct_target_into(&self, out: &mut [f32]) {
        let block_count = usize::try_from(self.block_count).unwrap_or(0);
        for (dct, out_block) in self
            .dct_coefs
            .iter()
            .zip(out.as_chunks_mut::<64>().0.iter_mut())
            .take(block_count)
        {
            dct.mul(self.quant_table).write_to(out_block);
        }
    }
}

impl Coef for SIMDCoef {
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
    fn clamp_dct_block(&self, block_idx: usize, data: &mut [f32]) {
        use std::simd::num::SimdFloat;
        if let (Some(&max), Some(&min)) = (
            self.dequant_dct_coefs_max.get(block_idx),
            self.dequant_dct_coefs_min.get(block_idx),
        ) {
            f32x64::from_slice(data).simd_clamp(min, max).write_to(data);
        }
    }
}
