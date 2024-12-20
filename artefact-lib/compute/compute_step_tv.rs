use super::aux::Aux;

/// Computes the Total Variation (TV) regularization term and its gradient
///
/// This function:
/// 1. Calculates forward differences in x and y directions for each channel
/// 2. Computes the TV norm of the gradients
/// 3. Updates the objective function gradient
/// 4. Stores gradient components for potential second-order TV (TGV) computation
///
/// # Arguments
/// * `w` - Width of the image
/// * `h` - Height of the image
/// * `nchannel` - Number of color channels
/// * `auxs` - Array of auxiliary data structures containing image data and working buffers
///
/// # Returns
/// Total variation value for the objective function
pub fn compute_step_tv(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
) -> f64 {
    let mut tv = 0.0;

    for y in 0..max_rounded_px_h {
        for x in 0..max_rounded_px_w {
            compute_step_tv_inner(
                max_rounded_px_w,
                max_rounded_px_h,
                nchannel,
                auxs,
                x,
                y,
                &mut tv,
            );
        }
    }

    tv
}

fn compute_step_tv_inner(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    nchannel: usize,
    auxs: &mut [Aux],
    curr_x: u32,
    curr_y: u32,
    tv: &mut f64,
) {
    let mut g_xs = [0.0; 3];
    let mut g_ys = [0.0; 3];

    for c in 0..nchannel {
        let aux = &mut auxs[c];

        // forward difference x
        g_xs[c] = match curr_x {
            x if x >= max_rounded_px_w - 1 => 0.0,
            _ => {
                let next_idx = (curr_y * max_rounded_px_w + (curr_x + 1)) as usize;
                let prev_idx = (curr_y * max_rounded_px_w + curr_x) as usize;

                aux.fdata[next_idx] - aux.fdata[prev_idx]
            }
        };

        // forward difference y
        g_ys[c] = match curr_y {
            y if y >= max_rounded_px_h - 1 => 0.0,
            _ => {
                let next_idx = ((curr_y + 1) * max_rounded_px_w + curr_x) as usize;
                let prev_idx = (curr_y * max_rounded_px_w + curr_x) as usize;

                aux.fdata[next_idx] - aux.fdata[prev_idx]
            }
        };
    }

    // norm
    let mut g_norm = 0.0;
    for c in 0..nchannel {
        g_norm += g_xs[c].powi(2);
        g_norm += g_ys[c].powi(2);
    }
    g_norm = g_norm.sqrt();
    let alpha = 1.0 / (nchannel as f32).sqrt();
    *tv += alpha as f64 * g_norm as f64;

    // compute derivatives
    if g_norm != 0.0 {
        for c in 0..nchannel {
            auxs[c].obj_gradient[(curr_y * max_rounded_px_w + curr_x) as usize] +=
                alpha * -(g_xs[c] + g_ys[c]) / g_norm;

            if curr_x < max_rounded_px_w - 1 {
                auxs[c].obj_gradient[(curr_y * max_rounded_px_w + (curr_x + 1)) as usize] +=
                    alpha * g_xs[c] / g_norm;
            }

            if curr_y < max_rounded_px_h - 1 {
                auxs[c].obj_gradient[((curr_y + 1) * max_rounded_px_w + curr_x) as usize] +=
                    alpha * g_ys[c] / g_norm;
            }
        }
    }

    // store for use in tv2
    for c in 0..nchannel {
        auxs[c].pixel_diff.x[(curr_y * max_rounded_px_w + curr_x) as usize] = g_xs[c];
        auxs[c].pixel_diff.y[(curr_y * max_rounded_px_w + curr_x) as usize] = g_ys[c];
    }
}
