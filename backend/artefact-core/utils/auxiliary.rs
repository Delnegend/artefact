#[derive(Debug)]
pub struct PixelDifference {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}

/// Working buffers for each component
#[derive(Debug)]
pub struct Aux {
    /// DCT coefficients for `step_prob`
    pub cos: Vec<f32>,

    /// Gradient (derivative) of the objective function
    pub obj_gradient: Vec<f32>,

    pub pixel_diff: PixelDifference,

    /// Image data
    pub fdata: Vec<f32>,

    /// Previous step image data for FISTA
    pub fista: Vec<f32>,
}

pub trait AuxTraits {
    /// Number of elements [`AuxTraits::get_cos`] writes (the component's rounded
    /// pixel count, i.e. `block_count * 64`).
    fn cos_count(&self) -> usize;

    /// Upsample this component's `image_data` into `out` (length
    /// `max_rounded_px_count`).
    fn get_fdata(&self, max_rounded_px_w: u32, max_rounded_px_h: u32, out: &mut [f32]);

    /// Write the dequantized DCT coefficients into `out` (length
    /// [`AuxTraits::cos_count`]).
    fn get_cos(&self, out: &mut [f32]);
}

/// Nearest-neighbour upsample of a subsampled raster (`image_data`) to the full
/// rounded resolution, replicating each sample by the component's sampling factors.
pub fn upsample_fdata(
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
        let mut fdata = vec![0.0; max_rounded_px_count];
        coef.get_fdata(max_rounded_px_w, max_rounded_px_h, &mut fdata);

        let mut cos = vec![0.0; coef.cos_count()];
        coef.get_cos(&mut cos);

        Self {
            cos,
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
