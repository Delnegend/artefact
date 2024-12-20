use wide::f32x8;

use super::dct::*;

pub fn idct8x8s(a: &mut [f32; 64]) {
    let c8_1r = f32x8::splat(C8_1R);
    let c8_1i = f32x8::splat(C8_1I);
    let c8_2r = f32x8::splat(C8_2R);
    let c8_2i = f32x8::splat(C8_2I);
    let c8_3r = f32x8::splat(C8_3R);
    let c8_3i = f32x8::splat(C8_3I);
    let c8_4r = f32x8::splat(C8_4R);
    let w8_4r = f32x8::splat(W8_4R);

    for j in 0..8 {
        let mut x1r = c8_1r * f32x8::from_slice(&a[1 * 8 + j..1 * 8 + j + 8])
            + c8_1i * f32x8::from_slice(&a[7 * 8 + j..7 * 8 + j + 8]);
        let mut x1i = c8_1r * f32x8::from_slice(&a[7 * 8 + j..7 * 8 + j + 8])
            - c8_1i * f32x8::from_slice(&a[1 * 8 + j..1 * 8 + j + 8]);
        let mut x3r = c8_3r * f32x8::from_slice(&a[3 * 8 + j..3 * 8 + j + 8])
            + c8_3i * f32x8::from_slice(&a[5 * 8 + j..5 * 8 + j + 8]);
        let mut x3i = c8_3r * f32x8::from_slice(&a[5 * 8 + j..5 * 8 + j + 8])
            - c8_3i * f32x8::from_slice(&a[3 * 8 + j..3 * 8 + j + 8]);
        let mut xr = x1r - x3r;
        let mut xi = x1i + x3i;
        x1r += x3r;
        x3i -= x1i;
        x1i = w8_4r * (xr + xi);
        x3r = w8_4r * (xr - xi);
        xr = c8_2r * f32x8::from_slice(&a[2 * 8 + j..2 * 8 + j + 8])
            + c8_2i * f32x8::from_slice(&a[6 * 8 + j..6 * 8 + j + 8]);
        xi = c8_2r * f32x8::from_slice(&a[6 * 8 + j..6 * 8 + j + 8])
            - c8_2i * f32x8::from_slice(&a[2 * 8 + j..2 * 8 + j + 8]);
        let mut x0r = c8_4r
            * (f32x8::from_slice(&a[0 * 8 + j..0 * 8 + j + 8])
                + f32x8::from_slice(&a[4 * 8 + j..4 * 8 + j + 8]));
        let mut x0i = c8_4r
            * (f32x8::from_slice(&a[0 * 8 + j..0 * 8 + j + 8])
                - f32x8::from_slice(&a[4 * 8 + j..4 * 8 + j + 8]));
        let mut x2r = x0r - xr;
        let mut x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        f32x8::store(x0r + x1r, &mut a[0 * 8 + j..0 * 8 + j + 8]);
        f32x8::store(x0r - x1r, &mut a[7 * 8 + j..7 * 8 + j + 8]);
        f32x8::store(x0i + x1i, &mut a[2 * 8 + j..2 * 8 + j + 8]);
        f32x8::store(x0i - x1i, &mut a[5 * 8 + j..5 * 8 + j + 8]);
        f32x8::store(x2r - x3i, &mut a[4 * 8 + j..4 * 8 + j + 8]);
        f32x8::store(x2r + x3i, &mut a[3 * 8 + j..3 * 8 + j + 8]);
        f32x8::store(x2i - x3r, &mut a[6 * 8 + j..6 * 8 + j + 8]);
        f32x8::store(x2i + x3r, &mut a[1 * 8 + j..1 * 8 + j + 8]);
    }
    for j in 0..8 {
        let mut x1r = c8_1r * f32x8::from_slice(&a[j * 8 + 1..j * 8 + 1 + 8])
            + c8_1i * f32x8::from_slice(&a[j * 8 + 7..j * 8 + 7 + 8]);
        let mut x1i = c8_1r * f32x8::from_slice(&a[j * 8 + 7..j * 8 + 7 + 8])
            - c8_1i * f32x8::from_slice(&a[j * 8 + 1..j * 8 + 1 + 8]);
        let mut x3r = c8_3r * f32x8::from_slice(&a[j * 8 + 3..j * 8 + 3 + 8])
            + c8_3i * f32x8::from_slice(&a[j * 8 + 5..j * 8 + 5 + 8]);
        let mut x3i = c8_3r * f32x8::from_slice(&a[j * 8 + 5..j * 8 + 5 + 8])
            - c8_3i * f32x8::from_slice(&a[j * 8 + 3..j * 8 + 3 + 8]);
        let mut xr = x1r - x3r;
        let mut xi = x1i + x3i;
        x1r += x3r;
        x3i -= x1i;
        x1i = w8_4r * (xr + xi);
        x3r = w8_4r * (xr - xi);
        xr = c8_2r * f32x8::from_slice(&a[j * 8 + 2..j * 8 + 2 + 8])
            + c8_2i * f32x8::from_slice(&a[j * 8 + 6..j * 8 + 6 + 8]);
        xi = c8_2r * f32x8::from_slice(&a[j * 8 + 6..j * 8 + 6 + 8])
            - c8_2i * f32x8::from_slice(&a[j * 8 + 2..j * 8 + 2 + 8]);
        let mut x0r = c8_4r
            * (f32x8::from_slice(&a[j * 8 + 0..j * 8 + 0 + 8])
                + f32x8::from_slice(&a[j * 8 + 4..j * 8 + 4 + 8]));
        let mut x0i = c8_4r
            * (f32x8::from_slice(&a[j * 8 + 0..j * 8 + 0 + 8])
                - f32x8::from_slice(&a[j * 8 + 4..j * 8 + 4 + 8]));
        let mut x2r = x0r - xr;
        let mut x2i = x0i - xi;
        x0r += xr;
        x0i += xi;
        f32x8::store(x0r + x1r, &mut a[j * 8 + 0..j * 8 + 0 + 8]);
        f32x8::store(x0r - x1r, &mut a[j * 8 + 7..j * 8 + 7 + 8]);
        f32x8::store(x0i + x1i, &mut a[j * 8 + 2..j * 8 + 2 + 8]);
        f32x8::store(x0i - x1i, &mut a[j * 8 + 5..j * 8 + 5 + 8]);
        f32x8::store(x2r - x3i, &mut a[j * 8 + 4..j * 8 + 4 + 8]);
        f32x8::store(x2r + x3i, &mut a[j * 8 + 3..j * 8 + 3 + 8]);
        f32x8::store(x2i - x3r, &mut a[j * 8 + 6..j * 8 + 6 + 8]);
        f32x8::store(x2i + x3r, &mut a[j * 8 + 1..j * 8 + 1 + 8]);
    }
}
