use zune_jpeg::sample_factor::SampleFactor;

#[cfg(feature = "mozjpeg")]
mod moz;
#[cfg(not(feature = "mozjpeg"))]
mod zune;

use crate::utils::{boxing::unboxing, dct::idct8x8s};
#[cfg(feature = "simd")]
use crate::{f32x8, traits::WriteTo};

#[derive(Debug, Clone)]
pub struct Coefficient {
    /// Rounded up until the next multiple of 8
    pub rounded_px_w: u32,
    /// Rounded up until the next multiple of 8
    pub rounded_px_h: u32,
    pub rounded_px_count: u32,

    /// Result after dividing the pixel width by 8 and rounding up
    pub block_w: u32,
    /// Result after dividing the pixel height by 8 and rounding up
    pub block_h: u32,
    pub block_count: u32,

    pub horizontal_samp_factor: SampleFactor,
    pub vertical_samp_factor: SampleFactor,

    #[cfg(not(feature = "simd"))]
    pub dct_coefs: Vec<f32>, // originally i16, but for convenience we use f32
    #[cfg(not(feature = "simd"))]
    pub quant_table: [f32; 64], // i32, same as above

    #[cfg(feature = "simd")]
    pub dct_coefs: Vec<f32x8>,
    #[cfg(feature = "simd")]
    pub quant_table: [f32x8; 8],
    #[cfg(feature = "simd")]
    pub quant_table_squared: [f32x8; 8],

    #[cfg(feature = "simd")]
    pub dequant_dct_coefs_min: Vec<f32x8>,
    #[cfg(feature = "simd")]
    pub dequant_dct_coefs_max: Vec<f32x8>,

    pub image_data: Vec<f32>,
}

impl Default for Coefficient {
    fn default() -> Self {
        Self {
            rounded_px_w: 0,
            rounded_px_h: 0,
            rounded_px_count: 0,
            block_w: 0,
            block_h: 0,
            block_count: 0,
            horizontal_samp_factor: SampleFactor::One,
            vertical_samp_factor: SampleFactor::One,

            #[cfg(not(feature = "simd"))]
            dct_coefs: vec![],

            #[cfg(feature = "simd")]
            dct_coefs: vec![f32x8::splat(0.0); 64],

            #[cfg(not(feature = "simd"))]
            quant_table: [0.0; 64],

            #[cfg(feature = "simd")]
            quant_table: [f32x8::splat(0.0); 8],
            #[cfg(feature = "simd")]
            quant_table_squared: [f32x8::splat(0.0); 8],

            #[cfg(feature = "simd")]
            dequant_dct_coefs_min: vec![f32x8::splat(0.0); 8],
            #[cfg(feature = "simd")]
            dequant_dct_coefs_max: vec![f32x8::splat(0.0); 8],

            image_data: vec![],
        }
    }
}

impl Coefficient {
    fn post_process(&mut self) {
        // DCT coefs + quantization table -> image data
        #[cfg(not(feature = "simd"))]
        for i in 0..(block_count as usize) {
            for j in 0..64 {
                coef.image_data[i * 64 + j] =
                    coef.dct_coefs[i * 64 + j] as f32 * coef.quant_table[j] as f32;
            }

            idct8x8s(
                coef.image_data[i * 64..(i + 1) * 64]
                    .as_mut()
                    .try_into()
                    .expect("Invalid coef's image data length"),
            );
        }

        #[cfg(feature = "simd")]
        for i in 0..(self.block_count as usize) {
            for j in 0..8 {
                let dct_coefs = self.dct_coefs[i * 8 + j];
                let quant_table = self.quant_table[j];
                let result = dct_coefs * quant_table;

                let idx = i * 64 + j * 8;
                result.write_to(&mut self.image_data[idx..idx + 8]);
            }

            idct8x8s(
                self.image_data[i * 64..(i + 1) * 64]
                    .as_mut()
                    .try_into()
                    .expect("Invalid coef's image data length"),
            );

            self.quant_table_squared = self
                .quant_table
                .iter()
                .map(|&x| x * x)
                .collect::<Vec<f32x8>>()
                .try_into()
                .expect("Invalid quant_table_squared length");
        }

        #[cfg(feature = "simd")]
        {
            self.dequant_dct_coefs_min = self
                .dct_coefs
                .iter()
                .enumerate()
                .map(|(idx, dct_coefs)| {
                    let quant_table = self.quant_table[idx % 8];
                    (*dct_coefs - f32x8::splat(0.5)) * quant_table
                })
                .collect();

            self.dequant_dct_coefs_max = self
                .dct_coefs
                .iter()
                .enumerate()
                .map(|(idx, dct_coefs)| {
                    let quant_table = self.quant_table[idx % 8];
                    (*dct_coefs + f32x8::splat(0.5)) * quant_table
                })
                .collect();
        }

        // 8x8 -> 64x1
        unboxing(
            &self.image_data.clone(),
            self.image_data.as_mut(),
            self.rounded_px_w,
            self.rounded_px_h,
            self.block_w,
            self.block_h,
        );
    }
}

#[derive(Debug)]
pub struct Jpeg {
    pub chan_count: u32,
    pub real_px_w: u32,
    pub real_px_h: u32,

    pub coefs: Vec<Coefficient>,
}

#[derive(Debug, Clone)]
pub enum JpegSource {
    File(String),
    Buffer(Vec<u8>),
}
