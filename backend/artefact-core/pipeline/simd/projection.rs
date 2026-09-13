use super::coef::Coef;
use crate::utils::{
    auxiliary::Aux,
    boxing::{boxing, unboxing},
    dct::{dct8x8s, idct8x8s},
};

/// Generic projection — resample to the subsampling grid, project each 8x8
/// block onto its quantized DCT box, then write back to pixels.
pub fn projection<C>(max_w: u32, max_h: u32, aux: &mut Aux, coef: &C)
where
    C: Coef,
{
    let resample = coef.rounded_px_w() != max_w || coef.rounded_px_h() != max_h;

    if resample {
        for cy in 0..coef.rounded_px_h() {
            for cx in 0..coef.rounded_px_w() {
                let mut mean = 0.0;
                for sy in 0..coef.vert_factor() {
                    for sx in 0..coef.horiz_factor() {
                        let y = cy * coef.vert_factor() + sy;
                        let x = cx * coef.horiz_factor() + sx;
                        debug_assert!(y < max_h && x < max_w);
                        mean += aux.fdata[(y * max_w + x) as usize];
                    }
                }
                mean /= f32::from((coef.horiz_factor() * coef.vert_factor()) as u8);
                aux.pixel_diff.y[(cy * coef.rounded_px_w() + cx) as usize] = mean;
                for sy in 0..coef.vert_factor() {
                    for sx in 0..coef.horiz_factor() {
                        let y = cy * coef.vert_factor() + sy;
                        let x = cx * coef.horiz_factor() + sx;
                        aux.fdata[(y * max_w + x) as usize] -= mean;
                    }
                }
            }
        }
    }

    boxing(
        if resample {
            &aux.pixel_diff.y
        } else {
            &aux.fdata
        },
        aux.pixel_diff.x.as_mut(),
        coef.rounded_px_w(),
        coef.rounded_px_h(),
        coef.block_w(),
        coef.block_h(),
    );

    for i in 0..coef.block_count() as usize {
        dct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    for i in 0..coef.block_count() as usize {
        coef.clamp_block(i, &mut aux.pixel_diff.x[i * 64..(i + 1) * 64]);
    }

    aux.cos.clone_from(&aux.pixel_diff.x);

    for i in 0..coef.block_count() as usize {
        idct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    unboxing(
        &aux.pixel_diff.x,
        if resample {
            aux.pixel_diff.y.as_mut()
        } else {
            aux.fdata.as_mut()
        },
        coef.rounded_px_w(),
        coef.rounded_px_h(),
        coef.block_w(),
        coef.block_h(),
    );

    if resample {
        for cy in 0..coef.rounded_px_h() {
            for cx in 0..coef.rounded_px_w() {
                let mean = aux.pixel_diff.y[(cy * coef.rounded_px_w() + cx) as usize];
                for sy in 0..coef.vert_factor() {
                    for sx in 0..coef.horiz_factor() {
                        let y = cy * coef.vert_factor() + sy;
                        let x = cx * coef.horiz_factor() + sx;
                        aux.fdata[(y * max_w + x) as usize] += mean;
                    }
                }
            }
        }
    }
}
