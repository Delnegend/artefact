use zune_jpeg::{zune_core::bytestream::ZCursor, JpegDecoder};

use crate::jpeg::{Coefficient, Jpeg, JpegSource};
#[cfg(feature = "simd")]
use crate::utils::{f32x8, traits::FromSlice};

impl Jpeg {
    pub fn from(jpeg_source: JpegSource) -> Result<Jpeg, String> {
        let buffer = match jpeg_source {
            JpegSource::File(path) => std::fs::read(path).map_err(|e| e.to_string())?,
            JpegSource::Buffer(buffer) => buffer,
        };

        let mut img = JpegDecoder::new(ZCursor::new(&buffer));
        img.decode().map_err(|e| e.to_string())?;

        let (real_px_w, real_px_h) = img
            .dimensions()
            .ok_or("No dimensions")
            .map(|(w, h)| (w as u32, h as u32))?;

        Ok(Jpeg {
            chan_count: img.components.len() as u32,
            real_px_w,
            real_px_h,
            coefs: {
                let mut coefs = Vec::with_capacity(img.components.len());

                for comp in img.components {
                    let block_w = u32::from(comp.rounded_px_w) / 8;
                    let block_h = u32::from(comp.rounded_px_h) / 8;
                    let block_count = block_w * block_h;

                    let mut coef = Coefficient {
                        rounded_px_w: u32::from(comp.rounded_px_w),
                        rounded_px_h: u32::from(comp.rounded_px_h),
                        rounded_px_count: comp.rounded_px_count as u32,
                        block_w,
                        block_h,
                        block_count,
                        horizontal_samp_factor: comp.horizontal_samp_factor,
                        vertical_samp_factor: comp.vertical_samp_factor,

                        #[cfg(not(feature = "simd"))]
                        dct_coefs: comp
                            .dct_coefs
                            .iter()
                            .map(|&x| f32::from(x))
                            .collect::<Vec<_>>(),

                        #[cfg(feature = "simd")]
                        dct_coefs: comp
                            .dct_coefs
                            .iter()
                            .map(|&x| f32::from(x))
                            .collect::<Vec<_>>()
                            .chunks_exact(8)
                            .map(f32x8::from_slc)
                            .collect(),

                        #[cfg(not(feature = "simd"))]
                        quant_table: comp
                            .quant_table
                            .iter()
                            .map(|&x| x as f32)
                            .collect::<Vec<_>>()
                            .try_into()
                            .map_err(|_| "Invalid quant_table_aligned length".to_string())?,

                        #[cfg(feature = "simd")]
                        quant_table: comp
                            .quant_table
                            .iter()
                            .map(|&x| x as f32)
                            .collect::<Vec<_>>()
                            .chunks_exact(8)
                            .map(f32x8::from_slc)
                            .collect::<Vec<f32x8>>()
                            .try_into()
                            .map_err(|_| "Invalid quant_table length".to_string())?,
                        #[cfg(feature = "simd")]
                        quant_table_squared: [f32x8::splat(0.0); 8],

                        #[cfg(feature = "simd")]
                        dequant_dct_coefs_min: vec![f32x8::splat(0.0); comp.rounded_px_count / 8],
                        #[cfg(feature = "simd")]
                        dequant_dct_coefs_max: vec![f32x8::splat(0.0); comp.rounded_px_count / 8],

                        image_data: vec![0.0; comp.rounded_px_count],
                    };
                    coef.post_process();
                    coefs.push(coef);
                }
                coefs
            },
        })
    }
}
