pub mod boxing;
pub mod dct;
pub mod macros;
#[cfg(feature = "simd")]
pub mod traits;

#[cfg(all(feature = "simd", feature = "simd_std"))]
pub use std::simd::f32x8;
#[cfg(all(feature = "simd", not(feature = "simd_std")))]
pub use wide::f32x8;
