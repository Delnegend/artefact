use crate::jpeg::{Coefficient, Jpeg, JpegSource};
use zune_jpeg::{zune_core::bytestream::ZCursor, JpegDecoder};

impl Jpeg {
    pub fn from(jpeg_source: JpegSource) -> Result<Self, String> {
        let buffer = match jpeg_source {
            JpegSource::File(path) => std::fs::read(&path)
                .map_err(|e| format!("Failed to read JPEG file '{path}': {e}"))?,
            JpegSource::Buffer(buffer) => buffer,
        };

        let mut img = JpegDecoder::new(ZCursor::new(&buffer));
        img.decode()
            .map_err(|e| format!("Failed to decode JPEG: {e}"))?;

        let (real_px_w, real_px_h) = img
            .dimensions()
            .map(|(w, h)| (w as u32, h as u32))
            .expect("Failed to get dimensions");

        let nchannel = img.components.len();

        let mut coefs = Vec::with_capacity(nchannel);

        for comp in img.components {
            coefs.push(Coefficient {
                rounded_px_w: u32::from(comp.rounded_px_w),
                rounded_px_h: u32::from(comp.rounded_px_h),
                rounded_px_count: comp.rounded_px_count as u32,
                block_w: u32::from(comp.rounded_px_w) / 8,
                block_h: u32::from(comp.rounded_px_h) / 8,
                block_count: (u32::from(comp.rounded_px_w) / 8)
                    * (u32::from(comp.rounded_px_h) / 8),
                horizontal_samp_factor: comp.horizontal_samp_factor,
                vertical_samp_factor: comp.vertical_samp_factor,

                dct_coefs: comp
                    .dct_coefs
                    .into_iter()
                    .map(f32::from)
                    .collect::<Vec<_>>(),

                quant_table: comp
                    .quant_table
                    .into_iter()
                    .map(|x| x as f32)
                    .collect::<Vec<_>>()
                    .try_into()
                    .map_err(|_| "Invalid quant_table length")?,
            });
        }

        Ok(Jpeg {
            nchannel: nchannel as u32,
            real_px_w,
            real_px_h,
            coefs,
        })
    }
}
