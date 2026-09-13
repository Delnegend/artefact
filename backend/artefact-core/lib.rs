#![feature(portable_simd)]
#![warn(clippy::perf, clippy::pedantic, clippy::nursery, clippy::unwrap_used)]
// wgpu's `Surface` auto-trait cycle overflows trait solving when the compiler
// computes `Send` for async fns that capture wgpu request descriptors.
#![allow(recursion_depth_exceeding_limit)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::cast_precision_loss,
    clippy::branches_sharing_code
)]

mod jpeg;
pub mod pipeline;
mod utils;

pub use pipeline::simd::traits::{FromSlice, SafeDiv, WriteTo};
pub use utils::aligned::AlignedF32;
pub use utils::auxiliary::{Aux, PixelDifference};

pub use image;
use rayon::prelude::*;

use jpeg::Jpeg;
pub use jpeg::JpegSource;
use utils::macros::mul_add;

// Single-crate dispatch — always buildable, no bloat via features
#[cfg(not(feature = "simd"))]
use pipeline::scalar::solve;
#[cfg(feature = "simd")]
use pipeline::simd::solve;

#[derive(Debug)]
pub enum ArtefactError {
    /// Returned by `Artefact::process` when `benchmark` mode is enabled.
    Benchmark,
    Message(String),
}

impl std::fmt::Display for ArtefactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Benchmark => write!(f, "benchmark mode"),
            Self::Message(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ArtefactError {}

impl From<String> for ArtefactError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

#[derive(Debug)]
pub enum ValueCollection<T> {
    ForAll(T),
    ForEach([T; 3]),
}

impl<T: Copy> ValueCollection<T> {
    const fn to_slice(&self) -> [T; 3] {
        match self {
            Self::ForAll(v) => [*v, *v, *v],
            Self::ForEach(v) => *v,
        }
    }
}

#[derive(Debug)]
pub struct Artefact {
    weight: ValueCollection<f32>,
    pweight: ValueCollection<f32>,
    iterations: ValueCollection<usize>,
    separate_components: bool,
    benchmark: bool,

    source: Option<JpegSource>,
}

impl Default for Artefact {
    fn default() -> Self {
        Self {
            weight: ValueCollection::ForAll(0.3),
            pweight: ValueCollection::ForAll(0.001),
            iterations: ValueCollection::ForAll(50),
            separate_components: false,
            benchmark: false,
            source: None,
        }
    }
}

macro_rules! define_methods {
    ($($name:ident: $t:ty),+) => {
        $(
            #[must_use] pub const fn $name(mut self, $name: $t) -> Self {
                self.$name = $name;
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
        weight: ValueCollection<f32>,
        pweight: ValueCollection<f32>,
        iterations: ValueCollection<usize>,
        benchmark: bool,
        separate_components: bool
    );

    /// Process the JPEG and return an RGB image buffer.
    /// If `benchmark` is set, returns [`ArtefactError::Benchmark`] instead.
    /// # Errors
    /// Returns [`ArtefactError::Message`] if the source is not set or the JPEG
    /// fails to decode, and [`ArtefactError::Benchmark`] when benchmarking.
    pub fn process(self) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError> {
        let jpeg = Jpeg::from(
            self.source
                .ok_or_else(|| ArtefactError::Message("source is not set".into()))?,
        )
        .map_err(|e| ArtefactError::Message(format!("failed to read JPEG: {e}")))?;
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
            solve(
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
                        &mut solve(
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

        if self.benchmark {
            return Err(ArtefactError::Benchmark);
        }

        // Fixup luma range for first channel
        for item in output[0].iter_mut().take(max_rounded_px_count) {
            *item += 128.0;
        }

        let max_w = max_rounded_px_w as usize;

        // YCbCr -> RGB
        if jpeg.nchannel == 3 {
            let (luma, cb, cr) = (&output[0], &output[1], &output[2]);
            return Ok(image::RgbImage::from_fn(
                jpeg.real_px_w,
                jpeg.real_px_h,
                |x, y| {
                    let idx = y as usize * max_w + x as usize;

                    let yi = luma[idx];
                    let cbi = cb[idx];
                    let cri = cr[idx];

                    image::Rgb([
                        mul_add!(1.402_f32, cri, yi).clamp(0.0, 255.0) as u8,
                        mul_add!(0.71414_f32, -cri, mul_add!(0.34414_f32, -cbi, yi))
                            .clamp(0.0, 255.0) as u8,
                        mul_add!(1.772_f32, cbi, yi).clamp(0.0, 255.0) as u8,
                    ])
                },
            ));
        }

        // Grayscale
        let luma = &output[0];
        Ok(image::RgbImage::from_fn(
            jpeg.real_px_w,
            jpeg.real_px_h,
            |x, y| {
                let v = luma[y as usize * max_w + x as usize].clamp(0.0, 255.0) as u8;
                image::Rgb([v, v, v])
            },
        ))
    }
}
