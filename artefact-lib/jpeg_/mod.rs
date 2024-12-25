#[cfg(feature = "mozjpeg")]
pub mod moz_decoder;
pub mod zune_decoder;

use std::ops::Deref;

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

    pub w_samp_factor: SampFactor,
    pub h_samp_factor: SampFactor,

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
