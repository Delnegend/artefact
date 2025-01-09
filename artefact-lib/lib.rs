mod compute;
mod jpeg;
mod utils;

pub use image;

use compute::compute;
use jpeg::Jpeg;
pub use jpeg::JpegSource;

#[derive(Debug)]
pub enum ValueCollection<T> {
    ForAll(T),
    ForEach([T; 3]),
}

impl<T: Copy> ValueCollection<T> {
    fn to_slice(&self) -> [T; 3] {
        match self {
            ValueCollection::ForAll(v) => [*v, *v, *v],
            ValueCollection::ForEach(v) => *v,
        }
    }
}

#[derive(Debug)]
pub struct Artefact {
    weight: ValueCollection<f32>,
    pweight: ValueCollection<f32>,
    iterations: ValueCollection<u32>,
    separate_components: bool,

    source: Option<JpegSource>,
}

impl Default for Artefact {
    fn default() -> Self {
        Self {
            weight: ValueCollection::ForAll(0.3),
            pweight: ValueCollection::ForAll(0.001),
            iterations: ValueCollection::ForAll(50),
            separate_components: false,
            source: None,
        }
    }
}

impl Artefact {
    pub fn source(mut self, source: JpegSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn weight(mut self, weight: Option<ValueCollection<f32>>) -> Self {
        if let Some(weight) = weight {
            self.weight = weight;
        };
        self
    }

    pub fn pweight(mut self, pweight: Option<ValueCollection<f32>>) -> Self {
        if let Some(pweight) = pweight {
            self.pweight = pweight;
        };
        self
    }

    pub fn iterations(mut self, iterations: Option<ValueCollection<u32>>) -> Self {
        if let Some(iterations) = iterations {
            self.iterations = iterations;
        };
        self
    }

    pub fn separate_components(mut self, separate_components: Option<bool>) -> Self {
        if let Some(separate_components) = separate_components {
            self.separate_components = separate_components;
        };
        self
    }

    pub fn process(self) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, String> {
        let jpeg = Jpeg::from(self.source.ok_or("Source is not set")?)?;
        let mut coefs = jpeg.coefs;

        let weight = self.weight.to_slice();
        let pweight = self.pweight.to_slice();
        let iterations = self.iterations.to_slice();

        // Smooth
        if jpeg.chan_count == 3 && !self.separate_components {
            compute(3, &mut coefs, weight[0], pweight, iterations[0]);
        } else {
            // Process channels separately
            for (c, coef) in coefs.iter().enumerate().take(jpeg.chan_count as usize) {
                let mut coef = vec![coef.clone()];
                compute(1, &mut coef, weight[c], pweight, iterations[c]);
            }
        }

        // Fixup luma range for first channel
        for i in 0..(coefs[0].rounded_px_h * coefs[0].rounded_px_w) as usize {
            coefs[0].image_data[i] += 128.0;
        }

        // YCbCr -> RGB
        if jpeg.chan_count == 3 {
            let mut rgb: Vec<[u8; 3]> =
                Vec::with_capacity((jpeg.real_px_h * jpeg.real_px_w) as usize);
            for i in 0..jpeg.real_px_h {
                for j in 0..jpeg.real_px_w {
                    let idx = (i * coefs[0].rounded_px_w + j) as usize;

                    let yi = coefs[0].image_data[idx];
                    let cbi = coefs[1].image_data[idx];
                    let cri = coefs[2].image_data[idx];

                    rgb.push([
                        (yi + 1.402 * cri).clamp(0.0, 255.0) as u8,
                        (yi - 0.34414 * cbi - 0.71414 * cri).clamp(0.0, 255.0) as u8,
                        (yi + 1.772 * cbi).clamp(0.0, 255.0) as u8,
                    ]);
                }
            }

            return Ok(image::RgbImage::from_fn(
                jpeg.real_px_w,
                jpeg.real_px_h,
                |x, y| {
                    let i = (y * jpeg.real_px_w + x) as usize;
                    image::Rgb(rgb[i])
                },
            ));
        }

        // Grayscale
        let mut gray: Vec<u8> = Vec::with_capacity((jpeg.real_px_h * jpeg.real_px_w) as usize);
        for i in 0..jpeg.real_px_h {
            for j in 0..jpeg.real_px_w {
                let idx = (i * coefs[0].rounded_px_w + j) as usize;
                gray.push(coefs[0].image_data[idx].clamp(0.0, 255.0) as u8);
            }
        }

        Ok(image::RgbImage::from_fn(
            jpeg.real_px_w,
            jpeg.real_px_h,
            |x, y| {
                let i = (y * jpeg.real_px_w + x) as usize;
                image::Rgb([gray[i], gray[i], gray[i]])
            },
        ))
    }
}
