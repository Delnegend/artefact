use crate::jpeg::{Coefficient, Jpeg, JpegSource};
use zune_jpeg::{
    JpegDecoder,
    zune_core::{bytestream::ZCursor, options::DecoderOptions},
};

impl Jpeg {
    pub fn from(jpeg_source: JpegSource) -> Result<Self, String> {
        let buffer = match jpeg_source {
            JpegSource::File(path) => std::fs::read(&path)
                .map_err(|e| format!("Failed to read JPEG file '{path}': {e}"))?,
            JpegSource::Buffer(buffer) => buffer,
        };

        // Strict mode so truncated/corrupt streams error out instead of
        // silently returning zeroed coefficients.
        let mut img = JpegDecoder::new_with_options(
            ZCursor::new(&buffer),
            DecoderOptions::default().set_strict_mode(true),
        );
        img.decode()
            .map_err(|e| format!("Failed to decode JPEG: {e}"))?;

        let (real_px_w, real_px_h) = img
            .dimensions()
            .ok_or_else(|| "JPEG headers were not decoded".to_string())?;

        let nchannel = img.components.len();
        if nchannel != 1 && nchannel != 3 {
            return Err(format!(
                "Unsupported number of components: {nchannel} (only grayscale and YCbCr are supported)"
            ));
        }

        let mut coefs = Vec::with_capacity(nchannel);

        for comp in img.components {
            let block_w = comp.rounded_px_w / 8;
            let block_h = comp.rounded_px_h / 8;

            coefs.push(Coefficient {
                rounded_px_w: comp.rounded_px_w,
                rounded_px_h: comp.rounded_px_h,
                rounded_px_count: comp.rounded_px_count as u32,
                block_w,
                block_h,
                block_count: block_w * block_h,
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

        Ok(Self {
            nchannel: nchannel as u32,
            real_px_w: real_px_w.into(),
            real_px_h: real_px_h.into(),
            coefs,
        })
    }
}
