//! Ooura DCT/IDCT (source: <http://www.kurims.kyoto-u.ac.jp/~ooura/fft.html>,
//! `fft2d.zip`, `shrtdct.c`; Copyright Takuya OOURA 1996-2001).
use crate::utils::macros::mul_add;

pub const C8_1R: f32 = 0.490_392_63_f32;
pub const C8_1I: f32 = 0.097_545_16_f32;
pub const C8_2R: f32 = 0.461_939_75_f32;
pub const C8_2I: f32 = 0.191_341_71_f32;
pub const C8_3R: f32 = 0.415_734_8_f32;
pub const C8_3I: f32 = 0.277_785_12_f32;
pub const C8_4R: f32 = 0.353_553_38_f32;
pub const W8_4R: f32 = std::f32::consts::FRAC_1_SQRT_2;

pub fn idct8x8(a: &mut [f32; 64]) {
    // Column pass (8 columns, stride 8) via row slices to avoid indexing/arithmetic lints
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
        let mut x1r = mul_add!(C8_1R, *a1, C8_1I * *a7);
        let mut x1i = mul_add!(C8_1R, *a7, -(C8_1I * *a1));
        let x3r_init = mul_add!(C8_3R, *a3, C8_3I * *a5);
        let mut x3i = mul_add!(C8_3R, *a5, -(C8_3I * *a3));
        let mut xr = x1r - x3r_init;
        let mut xi = x1i + x3i;
        x1r += x3r_init;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        let x3r = W8_4R * (xr - xi);
        xr = mul_add!(C8_2R, *a2, C8_2I * *a6);
        xi = mul_add!(C8_2R, *a6, -(C8_2I * *a2));
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
        let mut x1r = mul_add!(C8_1R, *a1, C8_1I * *a7);
        let mut x1i = mul_add!(C8_1R, *a7, -(C8_1I * *a1));
        let x3r_init = mul_add!(C8_3R, *a3, C8_3I * *a5);
        let mut x3i = mul_add!(C8_3R, *a5, -(C8_3I * *a3));
        let mut xr = x1r - x3r_init;
        let mut xi = x1i + x3i;
        x1r += x3r_init;
        x3i -= x1i;
        x1i = W8_4R * (xr + xi);
        let x3r = W8_4R * (xr - xi);
        xr = mul_add!(C8_2R, *a2, C8_2I * *a6);
        xi = mul_add!(C8_2R, *a6, -(C8_2I * *a2));
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

pub fn dct8x8(a: &mut [f32; 64]) {
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
        let x0r = *a0 + *a7;
        let mut x1r = *a0 - *a7;
        let x0i = *a2 + *a5;
        let mut x1i = *a2 - *a5;
        let x2r = *a4 + *a3;
        let mut x3r = *a4 - *a3;
        let x2i = *a6 + *a1;
        let mut x3i = *a6 - *a1;
        let mut xr = x0r + x2r;
        let mut xi = x0i + x2i;
        *a0 = C8_4R * (xr + xi);
        *a4 = C8_4R * (xr - xi);
        xr = x0r - x2r;
        xi = x0i - x2i;
        *a2 = mul_add!(C8_2R, xr, -(C8_2I * xi));
        *a6 = mul_add!(C8_2R, xi, C8_2I * xr);
        xr = W8_4R * (x1i - x3i);
        x1i = W8_4R * (x1i + x3i);
        x3i = x1i - x3r;
        x1i += x3r;
        x3r = x1r - xr;
        x1r += xr;
        *a1 = mul_add!(C8_1R, x1r, -(C8_1I * x1i));
        *a7 = mul_add!(C8_1R, x1i, C8_1I * x1r);
        *a3 = mul_add!(C8_3R, x3r, -(C8_3I * x3i));
        *a5 = mul_add!(C8_3R, x3i, C8_3I * x3r);
    }

    let (rows, _) = a.as_chunks_mut::<8>();
    for [a0, a1, a2, a3, a4, a5, a6, a7] in rows {
        let x0r = *a0 + *a7;
        let mut x1r = *a0 - *a7;
        let x0i = *a2 + *a5;
        let mut x1i = *a2 - *a5;
        let x2r = *a4 + *a3;
        let mut x3r = *a4 - *a3;
        let x2i = *a6 + *a1;
        let mut x3i = *a6 - *a1;
        let mut xr = x0r + x2r;
        let mut xi = x0i + x2i;
        *a0 = C8_4R * (xr + xi);
        *a4 = C8_4R * (xr - xi);
        xr = x0r - x2r;
        xi = x0i - x2i;
        *a2 = mul_add!(C8_2R, xr, -(C8_2I * xi));
        *a6 = mul_add!(C8_2R, xi, C8_2I * xr);
        xr = W8_4R * (x1i - x3i);
        x1i = W8_4R * (x1i + x3i);
        x3i = x1i - x3r;
        x1i += x3r;
        x3r = x1r - xr;
        x1r += xr;
        *a1 = mul_add!(C8_1R, x1r, -(C8_1I * x1i));
        *a7 = mul_add!(C8_1R, x1i, C8_1I * x1r);
        *a3 = mul_add!(C8_3R, x3r, -(C8_3I * x3i));
        *a5 = mul_add!(C8_3R, x3i, C8_3I * x3r);
    }
}
