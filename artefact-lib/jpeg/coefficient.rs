#[derive(Debug, Clone)]
pub struct Coefficient {
    /// Rounded up until the next multiple of 8
    pub rounded_px_w: u32,
    /// Rounded up until the next multiple of 8
    pub rounded_px_h: u32,
    pub rounded_px_count: u32,

    /// Result after dividing the pixel width by 8 and rounding up
    pub block_w: u32,
    /// Result after dividing the pixel height by 8 and rounding up
    pub block_h: u32,
    pub block_count: u32,

    /// Should be 1 or 2
    pub w_samp_factor: u32,
    /// Should be 1 or 2
    pub h_samp_factor: u32,

    pub dct_coefs: Vec<i16>,
    pub image_data: Vec<f32>,
    pub quant_table: [u16; 64],
}
