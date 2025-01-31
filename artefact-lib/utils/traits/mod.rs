use std::{
    ops::RangeInclusive,
    simd::{cmp::SimdPartialEq, num::SimdFloat, StdFloat},
};

pub trait WriteTo {
    fn write_to(&self, target: &mut [f32]);
    fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>);
}

impl WriteTo for wide::f32x8 {
    fn write_to(&self, target: &mut [f32]) {
        target.copy_from_slice(self.as_array_ref());
    }

    fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>) {
        target.copy_from_slice(&self.as_array_ref()[range]);
    }
}

impl WriteTo for std::simd::f32x8 {
    fn write_to(&self, target: &mut [f32]) {
        target.copy_from_slice(self.as_array());
    }
    fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>) {
        target.copy_from_slice(&self.as_array()[range]);
    }
}

pub trait FromSlice {
    /// Create a `f32x8` from a 8-element slice
    ///
    /// # Panics
    /// if the slice is not of length 8
    fn from_slc(slc: &[f32]) -> Self;

    /// Right-pad a `less-than-8-elem-slice` with `0`s to create a `f32x8`
    ///
    /// # Panics
    /// if the slice is longer than 8
    fn from_short_slc(slc: &[f32]) -> Self;

    /// Create a `f32x8` from a slice with a range
    ///
    /// # Panics
    /// if the range larger than 8 or out of bounds
    fn from_range_slc(slc: &[f32], range: RangeInclusive<usize>) -> Self;
}

impl FromSlice for wide::f32x8 {
    fn from_slc(slc: &[f32]) -> Self {
        wide::f32x8::from(slc)
    }
    fn from_short_slc(slc: &[f32]) -> Self {
        let mut tmp = [0.0; 8];
        tmp[..slc.len()].copy_from_slice(slc);
        wide::f32x8::from(tmp)
    }
    fn from_range_slc(slc: &[f32], range: RangeInclusive<usize>) -> Self {
        let mut tmp = [0.0; 8];
        tmp[range].copy_from_slice(slc);
        wide::f32x8::from(tmp)
    }
}

impl FromSlice for std::simd::f32x8 {
    fn from_slc(slice: &[f32]) -> Self {
        std::simd::f32x8::from_slice(slice)
    }
    fn from_short_slc(slice: &[f32]) -> Self {
        std::simd::f32x8::load_or_default(slice)
    }
    fn from_range_slc(slice: &[f32], range: RangeInclusive<usize>) -> Self {
        let mut tmp = std::simd::f32x8::splat(0.0);
        tmp[range].copy_from_slice(slice);
        tmp
    }
}

pub trait Clamp {
    fn clmp(&self, min: Self, max: Self) -> Self;
}

impl Clamp for wide::f32x8 {
    fn clmp(&self, min: Self, max: Self) -> Self {
        self.min(max).max(min)
    }
}

impl Clamp for std::simd::f32x8 {
    fn clmp(&self, min: Self, max: Self) -> Self {
        self.simd_clamp(min, max)
    }
}

pub trait SafeDiv {
    /// Perform element-wise division, but if the divisor is 0, the result is 0
    fn safe_div(&self, divisor: Self) -> Self;
}

impl SafeDiv for wide::f32x8 {
    fn safe_div(&self, divisor: Self) -> Self {
        match divisor.as_array_ref() {
            divisor if divisor.contains(&0.0) => wide::f32x8::from(
                divisor
                    .iter()
                    .enumerate()
                    .map(|(i, g_norm)| match g_norm {
                        0.0 => 0.0,
                        _ => self.as_array_ref()[i] / g_norm,
                    })
                    .collect::<Vec<f32>>()
                    .as_slice(),
            ),
            _ => *self / divisor,
        }
    }
}

impl SafeDiv for std::simd::f32x8 {
    fn safe_div(&self, divisor: Self) -> Self {
        use std::simd::f32x8;

        let mask = divisor.simd_ne(f32x8::splat(0.0));

        let mut tmp = [0.0; 8];
        (self / divisor).store_select(&mut tmp, mask);
        f32x8::from_slice(&tmp)
    }
}

pub trait AddSlice {
    fn add_slice(&self, slice: &[f32]) -> Self;
    fn add_short_slice(&self, slice: &[f32]) -> Self;
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self;
}

impl AddSlice for wide::f32x8 {
    fn add_slice(&self, slice: &[f32]) -> Self {
        *self + wide::f32x8::from_slc(slice)
    }
    fn add_short_slice(&self, slice: &[f32]) -> Self {
        *self + wide::f32x8::from_short_slc(slice)
    }
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self {
        *self + wide::f32x8::from_range_slc(slice, range)
    }
}

impl AddSlice for std::simd::f32x8 {
    fn add_slice(&self, slice: &[f32]) -> Self {
        *self + std::simd::f32x8::from_slc(slice)
    }
    fn add_short_slice(&self, slice: &[f32]) -> Self {
        *self + std::simd::f32x8::from_short_slc(slice)
    }
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self {
        *self + std::simd::f32x8::from_range_slc(slice, range)
    }
}

pub trait Squirt {
    fn squirt(&self) -> Self;
}

impl Squirt for wide::f32x8 {
    fn squirt(&self) -> Self {
        self.sqrt()
    }
}

impl Squirt for std::simd::f32x8 {
    fn squirt(&self) -> Self {
        self.sqrt()
    }
}
