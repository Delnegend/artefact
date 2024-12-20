pub mod coefficient;
pub mod decompressor;

use crate::jpeg::{
    coefficient::Coefficient,
    decompressor::{Decompressor, DecompressorErr, JpegSource},
};

#[derive(Debug)]
pub struct Jpeg {
    pub chan_count: u32,
    pub real_px_w: u32,
    pub real_px_h: u32,
    pub coefs: Vec<Coefficient>,
}

impl Jpeg {
    pub fn from(jpeg_source: JpegSource) -> Result<Self, DecompressorErr> {
        let mut decoder = Decompressor::new()?;

        decoder.set_source(jpeg_source)?;
        decoder.read_header()?;

        Ok(Jpeg {
            chan_count: decoder.num_components(),
            real_px_w: decoder.width(),
            real_px_h: decoder.height(),
            coefs: decoder.read_coefficients()?,
        })
    }
}
