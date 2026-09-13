use crate::utils::aligned::AlignedF32;

#[derive(Debug)]
pub struct PixelDifference {
    pub x: AlignedF32,
    pub y: AlignedF32,
}

/// Working buffers for each component
#[derive(Debug)]
pub struct Aux {
    /// Discrete Cosine Transform (DCT) coefficients for `dct_gradient`
    pub cos: AlignedF32,

    /// Gradient (derivative) of the objective function
    pub obj_gradient: AlignedF32,

    pub pixel_diff: PixelDifference,

    /// Image data
    pub fdata: AlignedF32,

    /// Previous step image data for FISTA
    pub fista: AlignedF32,
}

pub trait AuxTraits {
    /// Number of elements [`AuxTraits::init_dct_target_into`] writes (the component's rounded
    /// pixel count, i.e. `block_count * 64`).
    fn cos_count(&self) -> usize;

    /// Upsample this component's `image_data` into `out` (length
    /// `max_rounded_px_count`).
    fn init_pixels_into(&self, max_rounded_px_w: u32, max_rounded_px_h: u32, out: &mut [f32]);

    /// Write the dequantized DCT coefficients into `out` (length
    /// [`AuxTraits::cos_count`]).
    fn init_dct_target_into(&self, out: &mut [f32]);
}

/// Nearest-neighbour upsample of a subsampled raster (`image_data`) to the full
/// rounded resolution, replicating each sample by the component's sampling factors.
pub fn upsample_pixels(
    image_data: &[f32],
    rounded_px_w: u32,
    rounded_px_h: u32,
    horiz_factor: usize,
    vert_factor: usize,
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    out: &mut [f32],
) {
    for y in 0..max_rounded_px_h as usize {
        for x in 0..max_rounded_px_w as usize {
            let cy = (y / vert_factor).min(rounded_px_h as usize - 1);
            let cx = (x / horiz_factor).min(rounded_px_w as usize - 1);
            out[y * max_rounded_px_w as usize + x] = image_data[cy * rounded_px_w as usize + cx];
        }
    }
}

impl Aux {
    /// Init a new auxiliary buffer
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
        coef: &impl AuxTraits,
    ) -> Self {
        let mut fdata = AlignedF32::zeros(max_rounded_px_count);
        coef.init_pixels_into(max_rounded_px_w, max_rounded_px_h, &mut fdata);

        let mut cos = AlignedF32::zeros(coef.cos_count());
        coef.init_dct_target_into(&mut cos);

        Self {
            cos,
            obj_gradient: AlignedF32::zeros(max_rounded_px_count),

            pixel_diff: PixelDifference {
                x: AlignedF32::zeros(max_rounded_px_count),
                y: AlignedF32::zeros(max_rounded_px_count),
            },

            fista: fdata.clone(),
            fdata,
        }
    }
}
