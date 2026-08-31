#![allow(unused)]
#![allow(clippy::identity_op)]
#![allow(clippy::erasing_op)]
#![allow(clippy::excessive_precision)]

use criterion::Criterion;
use rand::Rng;
use wide::f32x8;

pub const C8_1R: f32 = 0.490_392_640_201_615_224_56;
pub const C8_1I: f32 = 0.097_545_161_008_064_133_92;
pub const C8_2R: f32 = 0.461_939_766_255_643_378_06;
pub const C8_2I: f32 = 0.191_341_716_182_544_885_86;
pub const C8_3R: f32 = 0.415_734_806_151_272_618_54;
pub const C8_3I: f32 = 0.277_785_116_509_801_112_37;
pub const C8_4R: f32 = 0.353_553_390_593_273_762_20;
pub const W8_4R: f32 = 0.707_106_781_186_547_524_40;

pub fn idct8x8s_simd(a: &mut [f32; 64]) {
    {
        let mut x0r = f32x8::splat(0.0);
        let mut x0i = f32x8::splat(0.0);
        let mut x1r = f32x8::splat(0.0);
        let mut x1i = f32x8::splat(0.0);
        let mut x2r = f32x8::splat(0.0);
        let mut x2i = f32x8::splat(0.0);
        let mut x3r = f32x8::splat(0.0);
        let mut x3i = f32x8::splat(0.0);
        let mut xr = f32x8::splat(0.0);
        let mut xi = f32x8::splat(0.0);

        x1r = C8_1R * f32x8::from(&a[1 * 8..1 * 8 + 8]) + C8_1I * f32x8::from(&a[7 * 8..7 * 8 + 8]);
        x1i = C8_1R * f32x8::from(&a[7 * 8..7 * 8 + 8]) - C8_1I * f32x8::from(&a[1 * 8..1 * 8 + 8]);
        x3r = C8_3R * f32x8::from(&a[3 * 8..3 * 8 + 8]) + C8_3I * f32x8::from(&a[5 * 8..5 * 8 + 8]);
        x3i = C8_3R * f32x8::from(&a[5 * 8..5 * 8 + 8]) - C8_3I * f32x8::from(&a[3 * 8..3 * 8 + 8]);
        xr = x1r - x3r;
        xi = x1i + x3i;
        x1r += x3r;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        x3r = W8_4R * (xr - xi);
        xr = C8_2R * f32x8::from(&a[2 * 8..2 * 8 + 8]) + C8_2I * f32x8::from(&a[6 * 8..6 * 8 + 8]);
        xi = C8_2R * f32x8::from(&a[6 * 8..6 * 8 + 8]) - C8_2I * f32x8::from(&a[2 * 8..2 * 8 + 8]);
        x0r = C8_4R * (f32x8::from(&a[0 * 8..0 * 8 + 8]) + f32x8::from(&a[4 * 8..4 * 8 + 8]));
        x0i = C8_4R * (f32x8::from(&a[0 * 8..0 * 8 + 8]) - f32x8::from(&a[4 * 8..4 * 8 + 8]));
        x2r = x0r - xr;
        x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        a[0 * 8..0 * 8 + 8].copy_from_slice((x0r + x1r).as_array_ref());
        a[7 * 8..7 * 8 + 8].copy_from_slice((x0r - x1r).as_array_ref());
        a[2 * 8..2 * 8 + 8].copy_from_slice((x0i + x1i).as_array_ref());
        a[5 * 8..5 * 8 + 8].copy_from_slice((x0i - x1i).as_array_ref());
        a[4 * 8..4 * 8 + 8].copy_from_slice((x2r - x3i).as_array_ref());
        a[3 * 8..3 * 8 + 8].copy_from_slice((x2r + x3i).as_array_ref());
        a[6 * 8..6 * 8 + 8].copy_from_slice((x2i - x3r).as_array_ref());
        a[1 * 8..1 * 8 + 8].copy_from_slice((x2i + x3r).as_array_ref());
    }

    {
        let mut x0r = 0.0;
        let mut x0i = 0.0;
        let mut x1r = 0.0;
        let mut x1i = 0.0;
        let mut x2r = 0.0;
        let mut x2i = 0.0;
        let mut x3r = 0.0;
        let mut x3i = 0.0;
        let mut xr = 0.0;
        let mut xi = 0.0;

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
}

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

pub fn dct_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct");
    let mut rng = rand::rng();

    let mut arr_a = [0.0; 64];
    arr_a.iter_mut().for_each(|x| *x = rng.random());

    let mut arr_b = arr_a.clone();

    group.bench_function("idct8x8s", |b| b.iter(|| idct8x8s(&mut arr_a)));

    let mut a = [0.0; 64];
    group.bench_function("idct8x8s_simd", |b| b.iter(|| idct8x8s_simd(&mut arr_b)));

    group.finish();
}
