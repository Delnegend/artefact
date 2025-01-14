pub mod compute_projection;
pub mod compute_step;
pub mod compute_step_prob;
pub mod compute_step_tv;
pub mod compute_step_tv2;
pub mod compute_step_tv_64;
pub mod compute_step_tv_par;

macro_rules! f32x8 {
    // Create a f32x8 from a slice with less than 8 elements
    ($fill_range:expr, $slice:expr) => {
        f32x8::from({
            let mut tmp = [0.0; 8];
            tmp[$fill_range].copy_from_slice(&$slice);
            tmp
        })
    };
    // Syntax sugar
    ($slice:expr) => {
        f32x8::from($slice)
    };
    // Syntax sugar
    () => {
        f32x8::splat(0.0)
    };
    // perform simd division if divisor doesn't contain 0 else scalar
    (div: $dividend:expr, $divisor:expr) => {{
        let dividend = $dividend;
        match $divisor.as_array_ref() {
            divisor if divisor.contains(&0.0) => f32x8::from(
                divisor
                    .iter()
                    .enumerate()
                    .map(|(i, g_norm)| match g_norm {
                        0.0 => 0.0,
                        _ => dividend.as_array_ref()[i] / g_norm,
                    })
                    .collect::<Vec<f32>>()
                    .as_slice(),
            ),
            _ => dividend / $divisor,
        }
    }};
}

pub(crate) use f32x8;
