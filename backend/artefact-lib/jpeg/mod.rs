#[cfg(feature = "moz")]
mod moz;
#[cfg(not(feature = "moz"))]
mod zune;

use zune_jpeg::sample_factor::SampleFactor;

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

    pub dct_coefs: Vec<f32>,
    pub quant_table: [f32; 64],
}

#[derive(Debug)]
pub struct Jpeg {
    pub nchannel: u32,
    pub real_px_w: u32,
    pub real_px_h: u32,

    pub coefs: Vec<Coefficient>,
}

#[derive(Debug, Clone)]
pub enum JpegSource {
    File(String),
    Buffer(Vec<u8>),
}
