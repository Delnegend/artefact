#![allow(unused)]

use std::{
    ops::RangeInclusive,
    simd::{cmp::SimdPartialEq, num::SimdFloat},
};

use paste::paste;

macro_rules! def_std_simd_type {
    ($($width:literal),+) => {
        paste! {
            $(
                type [<StdF32x $width>] = std::simd::[<f32x $width>];
            )+
        }
    };
}

def_std_simd_type!(8, 16, 32, 64);

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

macro_rules! gen_write_to {
    ($($width:literal),+) => {
        $(paste! {
            impl WriteTo for [<StdF32x $width>] {
                fn write_to(&self, target: &mut [f32]) {
                    target.copy_from_slice(self.as_array());
                }
                fn write_partial_to(&self, target: &mut [f32], range: RangeInclusive<usize>) {
                    target.copy_from_slice(&self.as_array()[range]);
                }
            }
        })+
    };
}

gen_write_to!(8, 16, 32, 64);

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
        Self::from(slc)
    }
    fn from_short_slc(slc: &[f32]) -> Self {
        let mut tmp = [0.0; 8];
        tmp[..slc.len()].copy_from_slice(slc);
        Self::from(tmp)
    }
    fn from_range_slc(slc: &[f32], range: RangeInclusive<usize>) -> Self {
        let mut tmp = [0.0; 8];
        tmp[range].copy_from_slice(slc);
        Self::from(tmp)
    }
}

macro_rules! gen_from_slice {
    ($($width:literal),+) => {
        $(paste! {
            impl FromSlice for [<StdF32x $width>] {
                fn from_slc(slice: &[f32]) -> Self {
                    std::simd::[<f32x $width>]::from_slice(slice)
                }
                fn from_short_slc(slice: &[f32]) -> Self {
                    std::simd::[<f32x $width>]::load_or_default(slice)
                }
                fn from_range_slc(slice: &[f32], range: RangeInclusive<usize>) -> Self {
                    let mut tmp = std::simd::[<f32x $width>]::splat(0.0);
                    tmp[range].copy_from_slice(slice);
                    tmp
                }
            }
        })+
    };
}

gen_from_slice!(8, 16, 32, 64);

pub trait Clamp {
    fn clmp(&self, min: Self, max: Self) -> Self;
}

impl Clamp for wide::f32x8 {
    fn clmp(&self, min: Self, max: Self) -> Self {
        self.min(max).max(min)
    }
}

macro_rules! gen_clamp {
    ($($width:literal),+) => {
        $(paste! {
            impl Clamp for [<StdF32x $width>] {
                fn clmp(&self, min: Self, max: Self) -> Self {
                    self.simd_clamp(min, max)
                }
            }
        })+
    };
}

gen_clamp!(8, 16, 32, 64);

pub trait SafeDiv {
    /// Perform element-wise division, but if the divisor is 0, the result is 0
    fn safe_div(&self, divisor: Self) -> Self;
}

impl SafeDiv for wide::f32x8 {
    fn safe_div(&self, divisor: Self) -> Self {
        match divisor.as_array_ref() {
            divisor if divisor.contains(&0.0) => Self::from(
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

macro_rules! gen_safe_div {
    ($($width:literal),+) => {
        $(paste! {
            impl SafeDiv for [<StdF32x $width>] {
                fn safe_div(&self, divisor: Self) -> Self {
                    use std::simd::[<f32x $width>];

                    let mask = divisor.simd_ne([<f32x $width>]::splat(0.0));

                    let mut tmp = [0.0; $width];
                    (self / divisor).store_select(&mut tmp, mask);
                    [<f32x $width>]::from_slice(&tmp)
                }
            }
        })+
    };
}

gen_safe_div!(8, 16, 32, 64);

pub trait AddSlice {
    fn add_slice(&self, slice: &[f32]) -> Self;
    fn add_short_slice(&self, slice: &[f32]) -> Self;
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self;
}

impl AddSlice for wide::f32x8 {
    fn add_slice(&self, slice: &[f32]) -> Self {
        *self + Self::from_slc(slice)
    }
    fn add_short_slice(&self, slice: &[f32]) -> Self {
        *self + Self::from_short_slc(slice)
    }
    fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self {
        *self + Self::from_range_slc(slice, range)
    }
}

macro_rules! gen_add_slice {
    ($($width:literal),+) => {
        $(paste! {
            impl AddSlice for [<StdF32x $width>] {
                fn add_slice(&self, slice: &[f32]) -> Self {
                    *self + [<StdF32x $width>]::from_slc(slice)
                }
                fn add_short_slice(&self, slice: &[f32]) -> Self {
                    *self + [<StdF32x $width>]::from_short_slc(slice)
                }
                fn add_range_slice(&self, slice: &[f32], range: RangeInclusive<usize>) -> Self {
                    *self + [<StdF32x $width>]::from_range_slc(slice, range)
                }
            }
        })+
    };
}

gen_add_slice!(8, 16, 32, 64);
