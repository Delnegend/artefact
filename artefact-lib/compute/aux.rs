use crate::jpeg::Coefficient;

#[derive(Debug)]
pub struct PixelDifference {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}

/// Working buffers for each component
#[derive(Debug)]
pub struct Aux {
    /// DCT coefficients for step_prob
    pub cos: Vec<f32>,

    /// Gradient (derivative) of the objective function
    pub obj_gradient: Vec<f32>,

    pub pixel_diff: PixelDifference,

    /// Image data
    pub fdata: Vec<f32>,

    /// Previous step image data for FISTA
    pub fista: Vec<f32>,
}

impl Aux {
    /// Init a new auxilary buffer
    ///
    /// # Arguments
    ///
    /// * `max_rounded_px_w` - Maximum rounded pixel width of the image
    /// * `max_rounded_px_h` - Maximum rounded pixel height of the image
    /// * `max_rounded_px_count` - 2 above values multiplied
    /// * `coef` - The coefficient data
    pub fn init(
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
        coef: &Coefficient,
    ) -> Self {
        let mut fdata = vec![0.0; max_rounded_px_count];

        for y in 0..max_rounded_px_h as usize {
            for x in 0..max_rounded_px_w as usize {
                let cy = (y / *coef.h_samp_factor as usize).min(coef.rounded_px_h as usize - 1);
                let cx = (x / *coef.w_samp_factor as usize).min(coef.rounded_px_w as usize - 1);

                let fdata_idx = y * max_rounded_px_w as usize + x;
                let img_data_idx = cy * coef.rounded_px_w as usize + cx;

                fdata[fdata_idx] = coef.image_data[img_data_idx];
            }
        }

        Self {
            cos: {
                let mut cos = Vec::with_capacity((coef.rounded_px_count) as usize);

                for i in 0..coef.block_count as usize {
                    for j in 0..64 {
                        cos.push(coef.dct_coefs[i * 64 + j] as f32 * coef.quant_table[j] as f32);
                    }
                }

                cos
            },

            obj_gradient: vec![0.0; max_rounded_px_count],

            pixel_diff: PixelDifference {
                x: vec![0.0; max_rounded_px_count],
                y: vec![0.0; max_rounded_px_count],
            },

            fista: fdata.clone(),
            fdata,
        }
    }
}
