//! Source: http://www.kurims.kyoto-u.ac.jp/~ooura/fft.html
//! fft2d.zip (2006/12/28) file shrtdct.c
//!
//! Copyright Takuya OOURA, 1996-2001
//!
//! You may use, copy, modify and distribute this code for any purpose
//! (include commercial use) and without fee. Please refer to this package
//! when you modify this code.
//!
//! Modifications:
//! double -> float
//! double indirection removed
//! added named functions instead of sign
//! added ASSUME_ALIGNED
//! rewrite in Rust

#![allow(clippy::identity_op)]
#![allow(clippy::erasing_op)]
#![allow(clippy::excessive_precision)]

// Cn_kR = sqrt(2.0/n) * cos(pi/2*k/n)
// Cn_kI = sqrt(2.0/n) * sin(pi/2*k/n)
// Wn_kR = cos(pi/2*k/n)
// Wn_kI = sin(pi/2*k/n)
pub const C8_1R: f32 = 0.490_392_640_201_615_224_56;
pub const C8_1I: f32 = 0.097_545_161_008_064_133_92;
pub const C8_2R: f32 = 0.461_939_766_255_643_378_06;
pub const C8_2I: f32 = 0.191_341_716_182_544_885_86;
pub const C8_3R: f32 = 0.415_734_806_151_272_618_54;
pub const C8_3I: f32 = 0.277_785_116_509_801_112_37;
pub const C8_4R: f32 = 0.353_553_390_593_273_762_20;
pub const W8_4R: f32 = 0.707_106_781_186_547_524_40;

/// Inverse Discrete Cosine Transform (IDCT) for 8x8 blocks
pub fn idct8x8s(a: &mut [f32; 64]) {
    let mut x0r: f32;
    let mut x0i: f32;
    let mut x1r: f32;
    let mut x1i: f32;
    let mut x2r: f32;
    let mut x2i: f32;
    let mut x3r: f32;
    let mut x3i: f32;
    let mut xr: f32;
    let mut xi: f32;

    for j in 0..8 {
        x1r = C8_1R * a[1 * 8 + j] + C8_1I * a[7 * 8 + j];
        x1i = C8_1R * a[7 * 8 + j] - C8_1I * a[1 * 8 + j];
        x3r = C8_3R * a[3 * 8 + j] + C8_3I * a[5 * 8 + j];
        x3i = C8_3R * a[5 * 8 + j] - C8_3I * a[3 * 8 + j];
        xr = x1r - x3r;
        xi = x1i + x3i;
        x1r += x3r;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        x3r = W8_4R * (xr - xi);
        xr = C8_2R * a[2 * 8 + j] + C8_2I * a[6 * 8 + j];
        xi = C8_2R * a[6 * 8 + j] - C8_2I * a[2 * 8 + j];
        x0r = C8_4R * (a[0 * 8 + j] + a[4 * 8 + j]);
        x0i = C8_4R * (a[0 * 8 + j] - a[4 * 8 + j]);
        x2r = x0r - xr;
        x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        a[0 * 8 + j] = x0r + x1r;
        a[7 * 8 + j] = x0r - x1r;
        a[2 * 8 + j] = x0i + x1i;
        a[5 * 8 + j] = x0i - x1i;
        a[4 * 8 + j] = x2r - x3i;
        a[3 * 8 + j] = x2r + x3i;
        a[6 * 8 + j] = x2i - x3r;
        a[1 * 8 + j] = x2i + x3r;
    }
    for j in 0..8 {
        x1r = C8_1R * a[j * 8 + 1] + C8_1I * a[j * 8 + 7];
        x1i = C8_1R * a[j * 8 + 7] - C8_1I * a[j * 8 + 1];
        x3r = C8_3R * a[j * 8 + 3] + C8_3I * a[j * 8 + 5];
        x3i = C8_3R * a[j * 8 + 5] - C8_3I * a[j * 8 + 3];
        xr = x1r - x3r;
        xi = x1i + x3i;
        x1r += x3r;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        x3r = W8_4R * (xr - xi);
        xr = C8_2R * a[j * 8 + 2] + C8_2I * a[j * 8 + 6];
        xi = C8_2R * a[j * 8 + 6] - C8_2I * a[j * 8 + 2];
        x0r = C8_4R * (a[j * 8 + 0] + a[j * 8 + 4]);
        x0i = C8_4R * (a[j * 8 + 0] - a[j * 8 + 4]);
        x2r = x0r - xr;
        x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        a[j * 8 + 0] = x0r + x1r;
        a[j * 8 + 7] = x0r - x1r;
        a[j * 8 + 2] = x0i + x1i;
        a[j * 8 + 5] = x0i - x1i;
        a[j * 8 + 4] = x2r - x3i;
        a[j * 8 + 3] = x2r + x3i;
        a[j * 8 + 6] = x2i - x3r;
        a[j * 8 + 1] = x2i + x3r;
    }
}

pub fn dct8x8s(a: &mut [f32; 64]) {
    let mut x0r: f32;
    let mut x0i: f32;
    let mut x1r: f32;
    let mut x1i: f32;
    let mut x2r: f32;
    let mut x2i: f32;
    let mut x3r: f32;
    let mut x3i: f32;
    let mut xr: f32;
    let mut xi: f32;

    for j in 0..8 {
        x0r = a[0 * 8 + j] + a[7 * 8 + j];
        x1r = a[0 * 8 + j] - a[7 * 8 + j];
        x0i = a[2 * 8 + j] + a[5 * 8 + j];
        x1i = a[2 * 8 + j] - a[5 * 8 + j];
        x2r = a[4 * 8 + j] + a[3 * 8 + j];
        x3r = a[4 * 8 + j] - a[3 * 8 + j];
        x2i = a[6 * 8 + j] + a[1 * 8 + j];
        x3i = a[6 * 8 + j] - a[1 * 8 + j];
        xr = x0r + x2r;
        xi = x0i + x2i;
        a[0 * 8 + j] = C8_4R * (xr + xi);
        a[4 * 8 + j] = C8_4R * (xr - xi);
        xr = x0r - x2r;
        xi = x0i - x2i;
        a[2 * 8 + j] = C8_2R * xr - C8_2I * xi;
        a[6 * 8 + j] = C8_2R * xi + C8_2I * xr;
        xr = W8_4R * (x1i - x3i);
        x1i = W8_4R * (x1i + x3i);
        x3i = x1i - x3r;
        x1i += x3r;
        x3r = x1r - xr;
        x1r += xr;
        a[1 * 8 + j] = C8_1R * x1r - C8_1I * x1i;
        a[7 * 8 + j] = C8_1R * x1i + C8_1I * x1r;
        a[3 * 8 + j] = C8_3R * x3r - C8_3I * x3i;
        a[5 * 8 + j] = C8_3R * x3i + C8_3I * x3r;
    }

    for j in 0..8 {
        x0r = a[j * 8 + 0] + a[j * 8 + 7];
        x1r = a[j * 8 + 0] - a[j * 8 + 7];
        x0i = a[j * 8 + 2] + a[j * 8 + 5];
        x1i = a[j * 8 + 2] - a[j * 8 + 5];
        x2r = a[j * 8 + 4] + a[j * 8 + 3];
        x3r = a[j * 8 + 4] - a[j * 8 + 3];
        x2i = a[j * 8 + 6] + a[j * 8 + 1];
        x3i = a[j * 8 + 6] - a[j * 8 + 1];
        xr = x0r + x2r;
        xi = x0i + x2i;
        a[j * 8 + 0] = C8_4R * (xr + xi);
        a[j * 8 + 4] = C8_4R * (xr - xi);
        xr = x0r - x2r;
        xi = x0i - x2i;
        a[j * 8 + 2] = C8_2R * xr - C8_2I * xi;
        a[j * 8 + 6] = C8_2R * xi + C8_2I * xr;
        xr = W8_4R * (x1i - x3i);
        x1i = W8_4R * (x1i + x3i);
        x3i = x1i - x3r;
        x1i += x3r;
        x3r = x1r - xr;
        x1r += xr;
        a[j * 8 + 1] = C8_1R * x1r - C8_1I * x1i;
        a[j * 8 + 7] = C8_1R * x1i + C8_1I * x1r;
        a[j * 8 + 3] = C8_3R * x3r - C8_3I * x3i;
        a[j * 8 + 5] = C8_3R * x3i + C8_3I * x3r;
    }
}
