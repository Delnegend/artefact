#![feature(portable_simd)]
#![warn(clippy::perf, clippy::pedantic, clippy::nursery, clippy::unwrap_used)]
#![allow(
    clippy::use_self,
    clippy::missing_const_for_fn,
    clippy::redundant_closure_for_method_calls,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::unused_self,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::trivially_copy_pass_by_ref,
    clippy::inefficient_to_string,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::cast_precision_loss,
    clippy::branches_sharing_code
)]

mod jpeg;
mod pipeline_scalar;
mod pipeline_simd_8;
mod pipeline_simd_adaptive;
mod utils;

pub use image;
use rayon::prelude::*;

use jpeg::Jpeg;
pub use jpeg::JpegSource;
use utils::macros::mul_add;

#[cfg(not(feature = "simd"))]
use pipeline_scalar::compute;
#[cfg(all(feature = "simd", not(feature = "simd_adaptive")))]
use pipeline_simd_8::compute;
#[cfg(all(feature = "simd", feature = "simd_adaptive"))]
use pipeline_simd_adaptive::compute;

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
    iterations: ValueCollection<usize>,
    separate_components: bool,

    source: Option<JpegSource>,
}

impl Default for Artefact {
    fn default() -> Self {
        Self {
            weight: ValueCollection::ForAll(0.3),
            pweight: ValueCollection::ForAll(0.001),
            iterations: ValueCollection::ForAll(100),
            separate_components: false,
            source: None,
        }
    }
}

macro_rules! define_methods {
    ($($name:ident: $t:ty),+) => {
        $(
            #[must_use] pub fn $name(mut self, $name: $t) -> Self {
                if let Some($name) = $name {
                    self.$name = $name;
                }
                self
            }
        )+
    }
}

impl Artefact {
    #[must_use]
    pub fn source(mut self, source: JpegSource) -> Self {
        self.source = Some(source);
        self
    }

    define_methods!(
        weight: Option<ValueCollection<f32>>,
        pweight: Option<ValueCollection<f32>>,
        iterations: Option<ValueCollection<usize>>,
        separate_components: Option<bool>
    );

    pub fn process(self) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, String> {
        let jpeg = Jpeg::from(self.source.ok_or("Source is not set")?)
            .map_err(|e| format!("Failed to read JPEG: {e}"))?;
        let (max_rounded_px_w, max_rounded_px_h, max_rounded_px_count) = {
            let mut w = 0;
            let mut h = 0;
            for coef in &jpeg.coefs {
                w = w.max(coef.rounded_px_w);
                h = h.max(coef.rounded_px_h);
            }
            (w, h, (w * h) as usize)
        };

        let weight = self.weight.to_slice();
        let pweight = self.pweight.to_slice();
        let iterations = self.iterations.to_slice();

        let mut output = if jpeg.nchannel == 3 && !self.separate_components {
            compute(
                3,
                jpeg.coefs,
                weight[0],
                pweight,
                iterations[0],
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
            )
        } else {
            // Process channels separately
            jpeg.coefs
                .into_par_iter()
                .enumerate()
                .map(|(c, coef)| {
                    std::mem::take(
                        &mut compute(
                            1,
                            vec![coef],
                            weight[c],
                            pweight,
                            iterations[c],
                            max_rounded_px_w,
                            max_rounded_px_h,
                            max_rounded_px_count,
                        )[0],
                    )
                })
                .collect::<Vec<_>>()
        };

        // Fixup luma range for first channel
        for i in 0..max_rounded_px_count {
            output[0][i] += 128.0;
        }

        // YCbCr -> RGB
        if jpeg.nchannel == 3 {
            let mut rgb: Vec<[u8; 3]> =
                Vec::with_capacity((jpeg.real_px_h * jpeg.real_px_w) as usize);
            for i in 0..jpeg.real_px_h {
                for j in 0..jpeg.real_px_w {
                    let idx = (i * max_rounded_px_w + j) as usize;

                    let yi = output[0][idx];
                    let cbi = output[1][idx];
                    let cri = output[2][idx];

                    rgb.push([
                        mul_add!(1.402_f32, cri, yi).clamp(0.0, 255.0) as u8,
                        mul_add!(0.71414_f32, -cri, mul_add!(0.34414_f32, -cbi, yi))
                            .clamp(0.0, 255.0) as u8,
                        mul_add!(1.772_f32, cbi, yi).clamp(0.0, 255.0) as u8,
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
                let idx = (i * max_rounded_px_w + j) as usize;
                gray.push(output[0][idx].clamp(0.0, 255.0) as u8);
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
