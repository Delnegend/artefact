use crate::{
    compute::{
        aux::Aux, compute_step_prob::compute_step_prob,
        compute_step_tv2_simd::compute_step_tv2_simd, compute_step_tv_simd::compute_step_tv_simd,
    },
    jpeg::Coefficient,
};

#[allow(clippy::too_many_arguments)]
pub fn compute_step(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
    nchannel: usize,
    coefs: &[Coefficient],
    auxs: &mut [Aux],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
) -> f64 {
    let mut total_alpha = 0.0;
    let mut prob_dist = 0.0;

    for c in 0..nchannel {
        let aux = &mut auxs[c];
        let coef = &coefs[c];

        aux.obj_gradient = vec![0.0; max_rounded_px_count];

        // DCT coefficient distance
        if pweight[c] != 0.0 {
            let p_alpha = pweight[c] * 2.0 * 255.0 * 2.0_f32.sqrt();
            total_alpha += p_alpha;
            prob_dist += compute_step_prob(
                max_rounded_px_w,
                max_rounded_px_h,
                p_alpha,
                coef,
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    }

    // TV computation
    total_alpha += nchannel as f32;

    let tv = compute_step_tv_simd(max_rounded_px_w, max_rounded_px_h, nchannel, auxs);

    // TGV second order
    let tv2 = match weight {
        0.0 => 0.0,
        _ => {
            let alpha = weight / 2.0_f32.sqrt();
            total_alpha += alpha * nchannel as f32;
            compute_step_tv2_simd(max_rounded_px_w, max_rounded_px_h, nchannel, auxs, alpha)
        }
    };

    // Performs a gradient descent step in the direction of the objective gradient
    // with a specified step size. The gradient is normalized before applying the step.
    for aux in auxs.iter_mut() {
        // Calculate Euclidean norm of the objective gradient
        let norm = {
            let mut norm = 0.0;
            for i in 0..max_rounded_px_count {
                norm += aux.obj_gradient[i].powi(2);
            }
            norm.sqrt()
        };

        // Only update if gradient norm is non-zero
        if norm != 0.0 {
            for i in 0..max_rounded_px_count {
                aux.fdata[i] -= step_size * (aux.obj_gradient[i] / norm);
            }
        }
    }

    // Calculate objective value
    (tv + tv2 + prob_dist) / total_alpha as f64
}
