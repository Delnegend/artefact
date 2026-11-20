use std::{
    ops::{Add, Mul, Sub},
    simd::f32x8,
};

use criterion::Criterion;
use rand::RngExt;

pub const C8_1R: f32 = 0.490_392_63_f32;
pub const C8_1I: f32 = 0.097_545_16_f32;
pub const C8_2R: f32 = 0.461_939_75_f32;
pub const C8_2I: f32 = 0.191_341_71_f32;
pub const C8_3R: f32 = 0.415_734_8_f32;
pub const C8_3I: f32 = 0.277_785_12_f32;
pub const C8_4R: f32 = 0.353_553_38_f32;
pub const W8_4R: f32 = std::f32::consts::FRAC_1_SQRT_2;

pub fn idct8x8_simd(a: &mut [f32; 64]) {
    {
        let (rows, _) = a.as_chunks::<8>();
        let [r0, r1, r2, r3, r4, r5, r6, r7] = rows else {
            return;
        };
        let a0 = f32x8::from_array(*r0);
        let a1 = f32x8::from_array(*r1);
        let a2 = f32x8::from_array(*r2);
        let a3 = f32x8::from_array(*r3);
        let a4 = f32x8::from_array(*r4);
        let a5 = f32x8::from_array(*r5);
        let a6 = f32x8::from_array(*r6);
        let a7 = f32x8::from_array(*r7);

        let mut x1r = f32x8::splat(C8_1R).mul(a1).add(f32x8::splat(C8_1I).mul(a7));
        let x1i_init = f32x8::splat(C8_1R).mul(a7).sub(f32x8::splat(C8_1I).mul(a1));
        let x3r_raw = f32x8::splat(C8_3R).mul(a3).add(f32x8::splat(C8_3I).mul(a5));
        let mut x3i = f32x8::splat(C8_3R).mul(a5).sub(f32x8::splat(C8_3I).mul(a3));
        let mut xr = x1r.sub(x3r_raw);
        let mut xi = x1i_init.add(x3i);
        x1r = x1r.add(x3r_raw);
        x3i = x3i.sub(x1i_init);
        let x1i = f32x8::splat(W8_4R).mul(xr.add(xi));
        let x3r = f32x8::splat(W8_4R).mul(xr.sub(xi));
        xr = f32x8::splat(C8_2R).mul(a2).add(f32x8::splat(C8_2I).mul(a6));
        xi = f32x8::splat(C8_2R).mul(a6).sub(f32x8::splat(C8_2I).mul(a2));
        let mut x0r = f32x8::splat(C8_4R).mul(a0.add(a4));
        let mut x0i = f32x8::splat(C8_4R).mul(a0.sub(a4));
        let x2r = x0r.sub(xr);
        let x2i = x0i.sub(xi);
        x0r = x0r.add(xr);
        x0i = x0i.add(xi);

        let (rows_mut, _) = a.as_chunks_mut::<8>();
        let [m0, m1, m2, m3, m4, m5, m6, m7] = rows_mut else {
            return;
        };
        *m0 = x0r.add(x1r).to_array();
        *m7 = x0r.sub(x1r).to_array();
        *m2 = x0i.add(x1i).to_array();
        *m5 = x0i.sub(x1i).to_array();
        *m4 = x2r.sub(x3i).to_array();
        *m3 = x2r.add(x3i).to_array();
        *m6 = x2i.sub(x3r).to_array();
        *m1 = x2i.add(x3r).to_array();
    }
    let (rows, _) = a.as_chunks_mut::<8>();
    for [a0, a1, a2, a3, a4, a5, a6, a7] in rows {
        let mut x1r = C8_1I.mul_add(*a7, C8_1R * *a1);
        let mut x1i = C8_1I.mul_add(-*a1, C8_1R * *a7);
        let x3r_init = C8_3I.mul_add(*a5, C8_3R * *a3);
        let mut x3i = C8_3I.mul_add(-*a3, C8_3R * *a5);
        let mut xr = x1r - x3r_init;
        let mut xi = x1i + x3i;
        x1r += x3r_init;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        let x3r = W8_4R * (xr - xi);
        xr = C8_2I.mul_add(*a6, C8_2R * *a2);
        xi = C8_2I.mul_add(-*a2, C8_2R * *a6);
        let mut x0r = C8_4R * (*a0 + *a4);
        let mut x0i = C8_4R * (*a0 - *a4);
        let x2r = x0r - xr;
        let x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        *a0 = x0r + x1r;
        *a7 = x0r - x1r;
        *a2 = x0i + x1i;
        *a5 = x0i - x1i;
        *a4 = x2r - x3i;
        *a3 = x2r + x3i;
        *a6 = x2i - x3r;
        *a1 = x2i + x3r;
    }
}

pub fn idct8x8(a: &mut [f32; 64]) {
    let (r0, rest) = a.split_at_mut(8);
    let (r1, rest) = rest.split_at_mut(8);
    let (r2, rest) = rest.split_at_mut(8);
    let (r3, rest) = rest.split_at_mut(8);
    let (r4, rest) = rest.split_at_mut(8);
    let (r5, rest) = rest.split_at_mut(8);
    let (r6, r7) = rest.split_at_mut(8);

    for (a0, (a1, (a2, (a3, (a4, (a5, (a6, a7))))))) in r0.iter_mut().zip(
        r1.iter_mut().zip(
            r2.iter_mut().zip(
                r3.iter_mut().zip(
                    r4.iter_mut()
                        .zip(r5.iter_mut().zip(r6.iter_mut().zip(r7.iter_mut()))),
                ),
            ),
        ),
    ) {
        let mut x1r = C8_1I.mul_add(*a7, C8_1R * *a1);
        let mut x1i = C8_1I.mul_add(-*a1, C8_1R * *a7);
        let x3r_init = C8_3I.mul_add(*a5, C8_3R * *a3);
        let mut x3i = C8_3I.mul_add(-*a3, C8_3R * *a5);
        let mut xr = x1r - x3r_init;
        let mut xi = x1i + x3i;
        x1r += x3r_init;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        let x3r = W8_4R * (xr - xi);
        xr = C8_2I.mul_add(*a6, C8_2R * *a2);
        xi = C8_2I.mul_add(-*a2, C8_2R * *a6);
        let mut x0r = C8_4R * (*a0 + *a4);
        let mut x0i = C8_4R * (*a0 - *a4);
        let x2r = x0r - xr;
        let x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        *a0 = x0r + x1r;
        *a7 = x0r - x1r;
        *a2 = x0i + x1i;
        *a5 = x0i - x1i;
        *a4 = x2r - x3i;
        *a3 = x2r + x3i;
        *a6 = x2i - x3r;
        *a1 = x2i + x3r;
    }

    let (rows, _) = a.as_chunks_mut::<8>();
    for [a0, a1, a2, a3, a4, a5, a6, a7] in rows {
        let mut x1r = C8_1I.mul_add(*a7, C8_1R * *a1);
        let mut x1i = C8_1I.mul_add(-*a1, C8_1R * *a7);
        let x3r_init = C8_3I.mul_add(*a5, C8_3R * *a3);
        let mut x3i = C8_3I.mul_add(-*a3, C8_3R * *a5);
        let mut xr = x1r - x3r_init;
        let mut xi = x1i + x3i;
        x1r += x3r_init;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        let x3r = W8_4R * (xr - xi);
        xr = C8_2I.mul_add(*a6, C8_2R * *a2);
        xi = C8_2I.mul_add(-*a2, C8_2R * *a6);
        let mut x0r = C8_4R * (*a0 + *a4);
        let mut x0i = C8_4R * (*a0 - *a4);
        let x2r = x0r - xr;
        let x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        *a0 = x0r + x1r;
        *a7 = x0r - x1r;
        *a2 = x0i + x1i;
        *a5 = x0i - x1i;
        *a4 = x2r - x3i;
        *a3 = x2r + x3i;
        *a6 = x2i - x3r;
        *a1 = x2i + x3r;
    }
}

pub fn dct_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct");
    let mut rng = rand::rng();

    let mut arr_a = [0.0; 64];
    for x in &mut arr_a {
        *x = rng.random();
    }

    let mut arr_b = arr_a;

    group.bench_function("idct8x8", |b| b.iter(|| idct8x8(&mut arr_a)));
    group.bench_function("idct8x8_simd", |b| b.iter(|| idct8x8_simd(&mut arr_b)));

    group.finish();
}
