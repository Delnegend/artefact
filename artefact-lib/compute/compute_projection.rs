use crate::{
    compute::aux::Aux,
    jpeg::Coefficient,
    utils::{
        boxing::{boxing, unboxing},
        dct::{dct8x8s, idct8x8s},
    },
};

pub fn compute_projection(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    aux: &mut Aux,
    coef: &Coefficient,
) {
    let resample = coef.rounded_px_w != max_rounded_px_w || coef.rounded_px_h != max_rounded_px_h;

    // downsample and keep the difference
    // more formally, decompose each subsampling block in the direction of our subsampling vector (a vector of ones)
    if resample {
        for cy in 0..coef.rounded_px_h {
            for cx in 0..coef.rounded_px_w {
                let mut mean = 0.0;
                for sy in 0..coef.vertical_samp_factor.u32() {
                    for sx in 0..coef.horizontal_samp_factor.u32() {
                        let y = cy * coef.vertical_samp_factor.u32() + sy;
                        let x = cx * coef.horizontal_samp_factor.u32() + sx;
                        assert!(y < max_rounded_px_h && x < max_rounded_px_w);
                        mean += aux.fdata[(y * max_rounded_px_w + x) as usize];
                    }
                }
                mean /= (coef.horizontal_samp_factor.u8() * coef.vertical_samp_factor.u8()) as f32;

                assert!(cx < coef.rounded_px_w && cy < coef.rounded_px_h);
                aux.pixel_diff.y[(cy * coef.rounded_px_w + cx) as usize] = mean;

                for sy in 0..coef.vertical_samp_factor.u32() {
                    for sx in 0..coef.horizontal_samp_factor.u32() {
                        let y = cy * coef.vertical_samp_factor.u32() + sy;
                        let x = cx * coef.horizontal_samp_factor.u32() + sx;

                        assert!(y < max_rounded_px_h && x < max_rounded_px_w);
                        aux.fdata[(y * max_rounded_px_w + x) as usize] -= mean;
                    }
                }
            }
        }
    }

    // Project onto DCT box
    boxing(
        match resample {
            true => &aux.pixel_diff.y,
            false => &aux.fdata,
        },
        aux.pixel_diff.x.as_mut(),
        coef.rounded_px_w,
        coef.rounded_px_h,
        coef.block_w,
        coef.block_h,
    );

    for i in 0..coef.block_count as usize {
        dct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    // Clamp DCT coefficients
    for i in 0..coef.block_count as usize {
        for j in 0..64 {
            let min = (coef.dct_coefs[i * 64 + j] as f32 - 0.5) * coef.quant_table[j] as f32;
            let max = (coef.dct_coefs[i * 64 + j] as f32 + 0.5) * coef.quant_table[j] as f32;
            aux.pixel_diff.x[i * 64 + j] = aux.pixel_diff.x[i * 64 + j].max(min).min(max);
        }
    }

    // Save a copy of the DCT values for step_prob
    aux.cos = aux.pixel_diff.x.clone();

    // add back the difference (orthogonal to our subsampling vector)
    for i in 0..coef.block_count as usize {
        idct8x8s(
            aux.pixel_diff.x[i * 64..(i + 1) * 64]
                .as_mut()
                .try_into()
                .expect("Invalid pixel difference data length"),
        );
    }

    unboxing(
        &aux.pixel_diff.x,
        match resample {
            true => aux.pixel_diff.y.as_mut(),
            false => aux.fdata.as_mut(),
        },
        coef.rounded_px_w,
        coef.rounded_px_h,
        coef.block_w,
        coef.block_h,
    );

    // Add back the difference
    if resample {
        for cy in 0..coef.rounded_px_h {
            for cx in 0..coef.rounded_px_w {
                let mean = aux.pixel_diff.y[(cy * coef.rounded_px_w + cx) as usize];
                for sy in 0..coef.vertical_samp_factor.u32() {
                    for sx in 0..coef.horizontal_samp_factor.u32() {
                        let y = cy * coef.vertical_samp_factor.u32() + sy;
                        let x = cx * coef.horizontal_samp_factor.u32() + sx;
                        aux.fdata[(y * max_rounded_px_w + x) as usize] += mean;
                    }
                }
            }
        }
    }
}
