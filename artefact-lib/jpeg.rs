use zune_jpeg::{sample_factor::SampleFactor, zune_core::bytestream::ZCursor, JpegDecoder};

use crate::utils::{boxing::unboxing, dct::idct8x8s};

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

    pub dct_coefs: Vec<f32>, // originally i16, but for convenience we use f32
    pub image_data: Vec<f32>,
    pub quant_table: [f32; 64], // i32, same as above
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
                    let block_w = comp.rounded_px_w as u32 / 8;
                    let block_h = comp.rounded_px_h as u32 / 8;
                    let block_count = block_w * block_h;

                    let mut coef = Coefficient {
                        rounded_px_w: comp.rounded_px_w as u32,
                        rounded_px_h: comp.rounded_px_h as u32,
                        rounded_px_count: comp.rounded_px_count as u32,
                        block_w,
                        block_h,
                        block_count,
                        horizontal_samp_factor: comp.horizontal_samp_factor,
                        vertical_samp_factor: comp.vertical_samp_factor,
                        dct_coefs: comp.dct_coefs.iter().map(|&x| x as f32).collect(),
                        image_data: vec![0.0; comp.rounded_px_count],
                        quant_table: comp
                            .quant_table
                            .iter()
                            .map(|&x| x as f32)
                            .collect::<Vec<_>>()
                            .try_into()
                            .map_err(|_| "Quantization table length is not 64")?,
                    };

                    // DCT coefs + quantization table -> image data
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

                    // 8x8 -> 64x1
                    unboxing(
                        &coef.image_data.clone(),
                        coef.image_data.as_mut(),
                        coef.rounded_px_w,
                        coef.rounded_px_h,
                        coef.block_w,
                        coef.block_h,
                    );

                    coefs.push(coef);
                }
                coefs
            },
        })
    }
}
