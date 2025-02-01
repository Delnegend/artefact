use crate::{
    jpeg::Coefficient,
    utils::{aux::AuxTraits, boxing::unboxing, dct::idct8x8s},
};
use zune_jpeg::sample_factor::SampleFactor;

pub struct ScalarCoef {
    pub rounded_px_w: u32,
    pub rounded_px_h: u32,
    pub rounded_px_count: u32,

    pub block_w: u32,
    pub block_h: u32,
    pub block_count: u32,

    pub horizontal_samp_factor: SampleFactor,
    pub vertical_samp_factor: SampleFactor,

    pub dct_coefs: Vec<f32>,
    pub quant_table: [f32; 64],
    pub image_data: Vec<f32>,
}

impl From<Coefficient> for ScalarCoef {
    fn from(c: Coefficient) -> Self {
        let mut image_data = vec![0.0; c.rounded_px_count as usize];

        // DCT coefs + quantization table -> image data
        for i in 0..(c.block_count as usize) {
            for j in 0..64 {
                image_data[i * 64 + j] = c.dct_coefs[i * 64 + j] * c.quant_table[j];
            }

            idct8x8s(
                image_data[i * 64..(i + 1) * 64]
                    .as_mut()
                    .try_into()
                    .expect("Invalid coef's image data length"),
            );

            // 8x8 -> 64x1
            unboxing(
                &image_data.clone(),
                image_data.as_mut(),
                c.rounded_px_w,
                c.rounded_px_h,
                c.block_w,
                c.block_h,
            );
        }

        Self {
            rounded_px_w: c.rounded_px_w,
            rounded_px_h: c.rounded_px_h,
            rounded_px_count: c.rounded_px_count,
            block_w: c.block_w,
            block_h: c.block_h,
            block_count: c.block_count,
            horizontal_samp_factor: c.horizontal_samp_factor,
            vertical_samp_factor: c.vertical_samp_factor,
            dct_coefs: c.dct_coefs,
            quant_table: c.quant_table,
            image_data,
        }
    }
}

impl AuxTraits for ScalarCoef {
    fn get_fdata(
        &self,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
    ) -> Vec<f32> {
        let mut fdata = vec![0.0; max_rounded_px_count];

        for y in 0..max_rounded_px_h as usize {
            for x in 0..max_rounded_px_w as usize {
                let cy =
                    (y / self.vertical_samp_factor.usize()).min(self.rounded_px_h as usize - 1);
                let cx =
                    (x / self.horizontal_samp_factor.usize()).min(self.rounded_px_w as usize - 1);

                let fdata_idx = y * max_rounded_px_w as usize + x;
                let img_data_idx = cy * self.rounded_px_w as usize + cx;

                fdata[fdata_idx] = self.image_data[img_data_idx];
            }
        }

        fdata
    }

    fn get_cos(&self) -> Vec<f32> {
        let mut cos = Vec::with_capacity((self.rounded_px_count) as usize);

        for i in 0..self.block_count as usize {
            for j in 0..64 {
                cos.push(self.dct_coefs[i * 64 + j] * self.quant_table[j]);
            }
        }

        cos
    }
}
