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
    fn from_slc(slc: &[f32]) -> Self;
    fn from_short_slc(slc: &[f32]) -> Self;
    fn from_range_slc(slc: &[f32], range: RangeInclusive<usize>) -> Self;
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
    #[must_use]
    fn safe_div(&self, divisor: Self) -> Self;
}

macro_rules! gen_safe_div {
    ($($width:literal),+) => {
        $(paste! {
            impl SafeDiv for [<StdF32x $width>] {
                fn safe_div(&self, divisor: Self) -> Self {
                    let mask = divisor.simd_ne(std::simd::[<f32x $width>]::splat(0.0));
                    let mut tmp = [0.0; $width];
                    (self / divisor).store_select(&mut tmp, mask);
                    std::simd::[<f32x $width>]::from_slice(&tmp)
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

pub trait SimdWidth: FromSlice + WriteTo + Clamp + SafeDiv + AddSlice + Copy {
    const WIDTH: usize;
    const PAD: usize;
}

impl SimdWidth for std::simd::f32x8 {
    const WIDTH: usize = 8;
    const PAD: usize = 0;
}
impl SimdWidth for std::simd::f32x16 {
    const WIDTH: usize = 16;
    const PAD: usize = 8;
}
impl SimdWidth for std::simd::f32x32 {
    const WIDTH: usize = 32;
    const PAD: usize = 24;
}
impl SimdWidth for std::simd::f32x64 {
    const WIDTH: usize = 64;
    const PAD: usize = 56;
}
