pub mod scalar;
pub mod simd;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(test)]
mod tests;
