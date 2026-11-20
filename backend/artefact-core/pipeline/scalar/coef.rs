use crate::{
    jpeg::Coefficient,
    utils::{auxiliary::AuxTraits, blocks::from_blocks, dct::idct8x8},
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
        let px_count = usize::try_from(c.rounded_px_count).unwrap_or(0);
        let mut image_data = vec![0.0; px_count];

        let block_count = usize::try_from(c.block_count).unwrap_or(0);
        for i in 0..block_count {
            if let Some(out_block) = image_data.as_chunks_mut::<64>().0.get_mut(i)
                && let Some(dct_block) = c.dct_coefs.as_chunks::<64>().0.get(i)
            {
                for ((out, &dct), &q) in out_block
                    .iter_mut()
                    .zip(dct_block.iter())
                    .zip(c.quant_table.iter())
                {
                    *out = dct * q;
                }

                idct8x8(out_block);
            }

            // 8x8 -> 64x1
            from_blocks(
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
    fn cos_count(&self) -> usize {
        usize::try_from(self.rounded_px_count).unwrap_or(0)
    }

    fn init_pixels_into(&self, max_rounded_px_w: u32, max_rounded_px_h: u32, out: &mut [f32]) {
        crate::utils::auxiliary::upsample_pixels(
            &self.image_data,
            self.rounded_px_w,
            self.rounded_px_h,
            self.horizontal_samp_factor.usize(),
            self.vertical_samp_factor.usize(),
            max_rounded_px_w,
            max_rounded_px_h,
            out,
        );
    }

    fn init_dct_target_into(&self, out: &mut [f32]) {
        let block_count = usize::try_from(self.block_count).unwrap_or(0);
        for (out_block, dct_block) in out
            .as_chunks_mut::<64>()
            .0
            .iter_mut()
            .zip(self.dct_coefs.as_chunks::<64>().0.iter())
            .take(block_count)
        {
            for ((o, &dct), &q) in out_block
                .iter_mut()
                .zip(dct_block.iter())
                .zip(self.quant_table.iter())
            {
                *o = dct * q;
            }
        }
    }
}
