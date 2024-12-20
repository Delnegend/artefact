mod compute;
mod jpeg;
mod utils;

use image::ImageBuffer;

use crate::{compute::compute, jpeg::Jpeg, utils::clamp::clamp};

pub use crate::jpeg::decompressor::{DecompressorErr, JpegSource};

#[derive(Debug)]
pub struct Config {
    /// Second order weight
    ///
    /// Higher values give smoother transitions with less staircasing
    pub weight: [f32; 3],

    /// Probability weight
    ///
    /// Higher values make the result more similar to the source JPEG
    pub pweight: [f32; 3],

    /// Iterations
    ///
    /// Higher values give better results but take more time
    pub iterations: [u32; 3],

    /// Separate components
    ///
    /// Separately optimize components instead of all together
    pub separate_components: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            weight: [0.3, 0.3, 0.3],
            pweight: [0.001, 0.001, 0.001],
            iterations: [50, 50, 50],
            separate_components: false,
        }
    }
}

pub fn pipeline(
    config: Option<Config>,
    jpeg_source: JpegSource,
) -> Result<ImageBuffer<image::Rgb<u8>, Vec<u8>>, DecompressorErr> {
    let config = config.unwrap_or_default();
    let jpeg = Jpeg::from(jpeg_source)?;
    let mut coefs = jpeg.coefs;

    // Smooth
    if jpeg.chan_count == 3 && !config.separate_components {
        compute(
            3,
            &mut coefs,
            config.weight[0],
            config.pweight,
            config.iterations[0],
        );
    } else {
        // Process channels separately
        for (c, coef) in coefs.iter().enumerate().take(jpeg.chan_count as usize) {
            let mut coef = vec![coef.clone()];
            compute(
                1,
                &mut coef,
                config.weight[c],
                config.pweight,
                config.iterations[c],
            );
        }
    }

    // Fixup luma range for first channel
    for i in 0..(coefs[0].rounded_px_h * coefs[0].rounded_px_w) as usize {
        coefs[0].image_data[i] += 128.0;
    }

    // YCbCr -> RGB
    let mut rgb: Vec<[u8; 3]> = Vec::with_capacity((jpeg.real_px_h * jpeg.real_px_w) as usize);
    for i in 0..jpeg.real_px_h {
        for j in 0..jpeg.real_px_w {
            let idx = (i * coefs[0].rounded_px_w + j) as usize;

            let yi = coefs[0].image_data[idx];
            let cbi = coefs[1].image_data[idx];
            let cri = coefs[2].image_data[idx];

            rgb.push([
                clamp(yi + 1.402 * cri),
                clamp(yi - 0.34414 * cbi - 0.71414 * cri),
                clamp(yi + 1.772 * cbi),
            ]);
        }
    }

    Ok(image::RgbImage::from_fn(
        jpeg.real_px_w,
        jpeg.real_px_h,
        |x, y| {
            let i = (y * jpeg.real_px_w + x) as usize;
            image::Rgb(rgb[i])
        },
    ))
}
