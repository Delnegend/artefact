use std::ops::Deref;

use zune_jpeg::{zune_core::bytestream::ZCursor, JpegDecoder};

use crate::utils::{boxing::unboxing, dct::idct8x8s};

#[derive(Debug, Clone)]
pub enum SampFactor {
    One,
    Two,
}

impl Deref for SampFactor {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        match self {
            SampFactor::One => &1,
            SampFactor::Two => &2,
        }
    }
}

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

    pub horizontal_samp_factor: SampFactor,
    pub vertical_samp_factor: SampFactor,

    pub dct_coefs: Vec<i16>,
    pub image_data: Vec<f32>,
    pub quant_table: [u16; 64],
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
                        horizontal_samp_factor: match comp.horizontal_samp_factor {
                            1 => SampFactor::One,
                            2 => SampFactor::Two,
                            _ => return Err("Invalid horizontal sample factor".to_string()),
                        },
                        vertical_samp_factor: match comp.vertical_samp_factor {
                            1 => SampFactor::One,
                            2 => SampFactor::Two,
                            _ => return Err("Invalid vertical sample factor".to_string()),
                        },
                        dct_coefs: comp.dct_coefs,
                        image_data: vec![0.0; comp.rounded_px_count],
                        quant_table: comp
                            .quant_table
                            .iter()
                            .map(|&x| x as u16)
                            .collect::<Vec<u16>>()
                            .try_into()
                            .map_err(|_| "Failed to convert quantization table".to_string())?,
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
