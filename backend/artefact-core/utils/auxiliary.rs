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
    fn get_fdata(
        &self,
        max_rounded_px_w: u32,
        max_rounded_px_h: u32,
        max_rounded_px_count: usize,
    ) -> Vec<f32>;

    fn get_cos(&self) -> Vec<f32>;
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
    max_rounded_px_count: usize,
) -> Vec<f32> {
    let mut fdata = vec![0.0; max_rounded_px_count];
    for y in 0..max_rounded_px_h as usize {
        for x in 0..max_rounded_px_w as usize {
            let cy = (y / vert_factor).min(rounded_px_h as usize - 1);
            let cx = (x / horiz_factor).min(rounded_px_w as usize - 1);
            fdata[y * max_rounded_px_w as usize + x] = image_data[cy * rounded_px_w as usize + cx];
        }
    }
    fdata
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
        let fdata = coef.get_fdata(max_rounded_px_w, max_rounded_px_h, max_rounded_px_count);

        Self {
            cos: coef.get_cos(),
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
