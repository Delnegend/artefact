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
    pub fn process(&self) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError> {
        let (jpeg, max_rounded_px_w, max_rounded_px_h, max_rounded_px_count) = self.decode()?;
        tracing::info!("solving on CPU pipeline");
        let output = self.solve_cpu(
            &jpeg,
            max_rounded_px_w,
            max_rounded_px_h,
            max_rounded_px_count,
        );
        self.finish(&jpeg, output, max_rounded_px_w, max_rounded_px_count)
    }

    /// Decode the source and compute the padded block-grid dimensions.
    fn decode(&self) -> Result<(Jpeg, u32, u32, usize), ArtefactError> {
        let jpeg = Jpeg::from(
            self.source
                .clone()
                .ok_or_else(|| ArtefactError::Message("source is not set".into()))?,
        )
        .map_err(|e| ArtefactError::Message(format!("failed to read JPEG: {e}")))?;

        let (mut w, mut h) = (0, 0);
        for coef in &jpeg.coefs {
            w = w.max(coef.rounded_px_w);
            h = h.max(coef.rounded_px_h);
        }
        Ok((jpeg, w, h, (w * h) as usize))
    }

    /// Run the CPU pipeline for the requested component split.
    fn solve_cpu(
        &self,
        jpeg: &Jpeg,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
    ) -> Vec<AlignedF32> {
        let weight = self.weight.to_slice();
        let pweight = self.pweight.to_slice();
        let iterations = self.iterations.to_slice();

        if jpeg.nchannel == 3 && !self.separate_components {
            solve(
                3,
                jpeg.coefs.clone(),
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
                .par_iter()
                .cloned()
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
        }
    }

    /// Turn the per-channel solved rasters into an RGB image.
    fn finish(
        &self,
        jpeg: &Jpeg,
        mut output: Vec<AlignedF32>,
        max_rounded_px_w: u32,
        max_rounded_px_count: usize,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError> {
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

#[cfg(feature = "gpu")]
impl Artefact {
    /// Solve on the GPU and return an RGB image buffer.
    ///
    /// # Errors
    /// Returns [`ArtefactError::Message`] if no GPU adapter is available, the
    /// JPEG fails to decode, or the GPU solver fails.
    // The wgpu request futures are `!Send` (see `pipeline::gpu`), so this is too.
    #[allow(clippy::future_not_send)]
    pub async fn process_gpu(
        &self,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError> {
        let ctx = pipeline::gpu::GpuContext::new()
            .await
            .map_err(|e| ArtefactError::Message(e.to_string()))?;
        let (jpeg, max_rounded_px_w, max_rounded_px_h, max_rounded_px_count) = self.decode()?;
        tracing::info!("solving on GPU");
        let output = self
            .solve_gpu(
                &ctx,
                &jpeg,
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
            )
            .await?;
        self.finish(&jpeg, output, max_rounded_px_w, max_rounded_px_count)
    }

    /// Try the GPU solver, falling back to the CPU pipeline when no adapter is
    /// available or the GPU path fails.
    ///
    /// # Errors
    /// Returns the CPU pipeline's error if both paths fail.
    #[allow(clippy::future_not_send)]
    pub async fn process_auto(
        &self,
    ) -> Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError> {
        // Only fall back when the GPU device or solve is unavailable; errors from
        // `finish` (e.g. benchmark mode) are propagated as-is.
        let ctx = match pipeline::gpu::GpuContext::new().await {
            Ok(ctx) => ctx,
            Err(e) => {
                tracing::warn!(error = %e, "no GPU adapter; falling back to CPU");
                return self.process();
            }
        };
        let (jpeg, w, h, count) = self.decode()?;
        match self.solve_gpu(&ctx, &jpeg, w, h, count).await {
            Ok(output) => {
                tracing::info!("solving on GPU");
                self.finish(&jpeg, output, w, count)
            }
            Err(e) => {
                tracing::warn!(error = %e, "GPU solve failed; falling back to CPU");
                self.process()
            }
        }
    }

    /// Run the GPU pipeline for the requested component split.
    async fn solve_gpu(
        &self,
        ctx: &pipeline::gpu::GpuContext,
        jpeg: &Jpeg,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
    ) -> Result<Vec<AlignedF32>, ArtefactError> {
        let weight = self.weight.to_slice();
        let pweight = self.pweight.to_slice();
        let iterations = self.iterations.to_slice();
        let to_error = |e: pipeline::gpu::GpuError| ArtefactError::Message(e.to_string());

        if jpeg.nchannel == 3 && !self.separate_components {
            pipeline::gpu::solve(
                ctx,
                &jpeg.coefs,
                weight[0],
                &pweight,
                iterations[0],
                max_rounded_px_w,
                max_rounded_px_h,
                max_rounded_px_count,
            )
            .await
            .map_err(to_error)
        } else {
            let mut output = Vec::with_capacity(jpeg.coefs.len());
            for (c, coef) in jpeg.coefs.iter().enumerate() {
                let mut solved = pipeline::gpu::solve(
                    ctx,
                    std::slice::from_ref(coef),
                    weight[c],
                    &pweight,
                    iterations[c],
                    max_rounded_px_w,
                    max_rounded_px_h,
                    max_rounded_px_count,
                )
                .await
                .map_err(to_error)?;
                output.push(solved.pop().ok_or_else(|| {
                    ArtefactError::Message("GPU solver returned no output".into())
                })?);
            }
            Ok(output)
        }
    }
}
