use std::{
    ops::RangeInclusive,
    simd::{Simd, cmp::SimdPartialEq},
};

pub trait WriteTo {
    fn write_to(&self, target: &mut [f32]);
    fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>);
}

pub trait FromSlice {
    fn from_range_slice(slc: &[f32], range: RangeInclusive<usize>) -> Self;
}

pub trait SafeDiv {
    /// Perform element-wise division, but if the divisor is 0, the result is 0
    #[must_use]
    fn safe_div(&self, divisor: Self) -> Self;
}

pub trait AddSlice {
    #[must_use]
    fn add_slice(&self, slice: &[f32]) -> Self;
    #[must_use]
    fn add_short_slice(&self, slice: &[f32]) -> Self;
    #[must_use]
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self;
}

// Blanket impls for every SIMD lane count, so the TV/TV2 kernels can be generic
// over `N` instead of macro-generated per width.
impl<const N: usize> WriteTo for Simd<f32, N> {
    fn write_to(&self, target: &mut [f32]) {
        target.copy_from_slice(self.as_array());
    }

    fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>) {
        target.copy_from_slice(&self.as_array()[range]);
    }
}

impl<const N: usize> FromSlice for Simd<f32, N> {
    fn from_range_slice(slice: &[f32], range: RangeInclusive<usize>) -> Self {
        let mut tmp = Self::splat(0.0);
        tmp[range].copy_from_slice(slice);
        tmp
    }
}

impl<const N: usize> SafeDiv for Simd<f32, N> {
    fn safe_div(&self, divisor: Self) -> Self {
        let mask = divisor.simd_ne(Self::splat(0.0));
        let mut tmp = [0.0; N];
        (self / divisor).store_select(&mut tmp, mask);
        Self::from_slice(&tmp)
    }
}

impl<const N: usize> AddSlice for Simd<f32, N> {
    fn add_slice(&self, slice: &[f32]) -> Self {
        *self + Self::from_slice(slice)
    }

    fn add_short_slice(&self, slice: &[f32]) -> Self {
        *self + Self::load_or_default(slice)
    }

    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self {
        *self + Self::from_range_slice(slice, range)
    }
}
