pub mod compute_projection;
pub mod compute_step;
pub mod compute_step_prob;
pub mod compute_step_tv;
pub mod compute_step_tv2;

macro_rules! f32x8 {
    // Create a f32x8 from a slice with less than 8 elements
    (shorter: $slice:expr) => {
        f32x8::load_or_default($slice)
    };
    // Same as above but allow to specify the range
    ($range:expr, $slice:expr) => {
        f32x8::from({
            let mut tmp = f32x8::splat(0.0);
            tmp[$range].copy_from_slice(&$slice);
            tmp
        })
    };
    // Syntax sugar
    (2) => {
        f32x8::splat(2.0)
    };
    (0.5) => {
        f32x8::splat(2.0)
    };
    ($slice:expr) => {
        f32x8::from_slice($slice)
    };
    () => {
        f32x8::splat(0.0)
    };
}

pub(crate) use f32x8;
