#![feature(portable_simd)]

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

// Single-crate dispatch - always buildable, no bloat via features
/// Convert a floating-point pixel value in `[0.0, 255.0]` to `u8` with
/// truncation (matching `v.clamp(0.0, 255.0) as u8` without `as` casts).
#[inline]
fn f32_to_u8_clamped(v: f32) -> u8 {
    // Fast IEEE-754 f32 -> u8 truncation for clamped [0.0, 255.0]
    let clamped = v.clamp(0.0, 255.0);
    if clamped.is_nan() || clamped < 1.0 {
        return 0;
    }
    let bits = clamped.to_bits();
    let exp = (bits >> 23) & 0xff;
    // clamped is in [1.0, 255.0], so exp is in [127, 134]
    let shift = 150_u32.saturating_sub(exp); // 150 = 127 + 23
    let mantissa = (bits & 0x007f_ffff) | 0x0080_0000;
    let int_val = mantissa.checked_shr(shift).unwrap_or(0);
    u8::try_from(int_val).unwrap_or(255)
}

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
        let output = self.solve_cpu(&jpeg, max_rounded_px_w, max_rounded_px_h);
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
        let max_px_count = usize::try_from(w)
            .ok()
            .and_then(|wu| usize::try_from(h).ok().and_then(|hu| wu.checked_mul(hu)))
            .ok_or_else(|| ArtefactError::Message("image dimensions overflow usize".into()))?;
        Ok((jpeg, w, h, max_px_count))
    }

    /// Run the CPU pipeline for the requested component split.
    fn solve_cpu(
        &self,
        jpeg: &Jpeg,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
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
            )
        } else {
            jpeg.coefs
                .par_iter()
                .cloned()
                .enumerate()
                .map(|(c, coef)| {
                    let w = weight.get(c).copied().unwrap_or(weight[0]);
                    let it = iterations.get(c).copied().unwrap_or(iterations[0]);
                    solve(
                        1,
                        vec![coef],
                        w,
                        pweight,
                        it,
                        max_rounded_px_w,
                        max_rounded_px_h,
                    )
                    .into_iter()
                    .next()
                    .unwrap_or_default()
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

        let Some(first) = output.first_mut() else {
            return Err(ArtefactError::Message("solver returned no output".into()));
        };
        for item in first.iter_mut().take(max_rounded_px_count) {
            *item += 128.0;
        }

        let max_w = usize::try_from(max_rounded_px_w)
            .map_err(|_| ArtefactError::Message("image width overflow".into()))?;

        if jpeg.nchannel == 3 {
            let [luma, cb, cr, ..] = output.as_slice() else {
                return Err(ArtefactError::Message(
                    "expected 3 output channels from solver".into(),
                ));
            };
            return Ok(image::RgbImage::from_fn(
                jpeg.real_px_w,
                jpeg.real_px_h,
                |x, y| {
                    let idx = usize::try_from(y)
                        .ok()
                        .and_then(|yu| yu.checked_mul(max_w))
                        .and_then(|row| usize::try_from(x).ok().and_then(|xu| row.checked_add(xu)));

                    let (yi, cbi, cri) = idx
                        .and_then(|i| {
                            Some((
                                luma.get(i).copied()?,
                                cb.get(i).copied()?,
                                cr.get(i).copied()?,
                            ))
                        })
                        .unwrap_or((0.0, 0.0, 0.0));

                    image::Rgb([
                        f32_to_u8_clamped(mul_add!(1.402_f32, cri, yi)),
                        f32_to_u8_clamped(mul_add!(
                            0.71414_f32,
                            -cri,
                            mul_add!(0.34414_f32, -cbi, yi)
                        )),
                        f32_to_u8_clamped(mul_add!(1.772_f32, cbi, yi)),
                    ])
                },
            ));
        }

        let luma = output
            .first()
            .ok_or_else(|| ArtefactError::Message("solver returned no output".into()))?;
        Ok(image::RgbImage::from_fn(
            jpeg.real_px_w,
            jpeg.real_px_h,
            |x, y| {
                let idx = usize::try_from(y)
                    .ok()
                    .and_then(|yu| yu.checked_mul(max_w))
                    .and_then(|row| usize::try_from(x).ok().and_then(|xu| row.checked_add(xu)));
                let v = f32_to_u8_clamped(idx.and_then(|i| luma.get(i).copied()).unwrap_or(0.0));
                image::Rgb([v, v, v])
            },
        ))
    }
}

#[cfg(feature = "gpu")]
type GpuProcessFuture<'a> = std::pin::Pin<
    Box<
        dyn Future<Output = Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>, ArtefactError>>
            + 'a,
    >,
>;

#[cfg(feature = "gpu")]
impl Artefact {
    /// Solve on the GPU and return an RGB image buffer.
    ///
    /// # Errors
    /// Returns [`ArtefactError::Message`] if no GPU adapter is available, the
    /// JPEG fails to decode, or the GPU solver fails.
    // The wgpu request futures are `!Send` (see `pipeline::gpu`), so this is boxed.
    pub fn process_gpu(&self) -> GpuProcessFuture<'_> {
        Box::pin(async move {
            let ctx = pipeline::gpu::GpuContext::new()
                .await
                .map_err(|e| ArtefactError::Message(e.to_string()))?;
            self.process_gpu_with(&ctx).await
        })
    }

    /// Solve with an existing GPU context and return an RGB image buffer.
    ///
    /// This keeps one GPU device, queue, and adapter selection outside the
    /// timed region for repeated solves or benchmarks.
    ///
    /// # Errors
    /// Returns [`ArtefactError::Message`] if the JPEG fails to decode or the
    /// GPU solver fails, and [`ArtefactError::Benchmark`] when benchmarking.
    pub fn process_gpu_with<'a>(
        &'a self,
        ctx: &'a pipeline::gpu::GpuContext,
    ) -> GpuProcessFuture<'a> {
        Box::pin(async move {
            let (jpeg, max_rounded_px_w, max_rounded_px_h, max_rounded_px_count) = self.decode()?;
            tracing::info!("solving on GPU");
            let output = self
                .solve_gpu(ctx, &jpeg, max_rounded_px_w, max_rounded_px_h)
                .await?;
            self.finish(&jpeg, output, max_rounded_px_w, max_rounded_px_count)
        })
    }

    /// Try the GPU solver, falling back to the CPU pipeline when no adapter is
    /// available or the GPU path fails.
    ///
    /// # Errors
    /// Returns the CPU pipeline's error if both paths fail.
    // The wgpu request futures are `!Send` (see `pipeline::gpu`), so this is boxed.
    pub fn process_auto(&self) -> GpuProcessFuture<'_> {
        Box::pin(async move {
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
            match self.solve_gpu(&ctx, &jpeg, w, h).await {
                Ok(output) => {
                    tracing::info!("solving on GPU");
                    self.finish(&jpeg, output, w, count)
                }
                Err(e) => {
                    tracing::warn!(error = %e, "GPU solve failed; falling back to CPU");
                    self.process()
                }
            }
        })
    }

    /// Run the GPU pipeline for the requested component split.
    fn solve_gpu<'a>(
        &'a self,
        ctx: &'a pipeline::gpu::GpuContext,
        jpeg: &'a Jpeg,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<AlignedF32>, ArtefactError>> + 'a>> {
        Box::pin(async move {
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
                )
                .await
                .map_err(to_error)
            } else {
                let mut output = Vec::with_capacity(jpeg.coefs.len());
                for (c, coef) in jpeg.coefs.iter().enumerate() {
                    let Some(&w) = weight.get(c) else {
                        return Err(ArtefactError::Message(format!(
                            "channel index {c} out of range"
                        )));
                    };
                    let Some(&it) = iterations.get(c) else {
                        return Err(ArtefactError::Message(format!(
                            "channel index {c} out of range"
                        )));
                    };
                    let mut solved = pipeline::gpu::solve(
                        ctx,
                        std::slice::from_ref(coef),
                        w,
                        &pweight,
                        it,
                        max_rounded_px_w,
                        max_rounded_px_h,
                    )
                    .await
                    .map_err(to_error)?;
                    output.push(solved.pop().ok_or_else(|| {
                        ArtefactError::Message("GPU solver returned no output".into())
                    })?);
                }
                Ok(output)
            }
        })
    }
}
