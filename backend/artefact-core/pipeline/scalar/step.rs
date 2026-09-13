use super::{
    coef::ScalarCoef, dct_gradient::dct_gradient, projection::project_onto_box, tgv::tgv_gradient,
    tv::tv_gradient,
};
use crate::utils::{aligned::AlignedF32, auxiliary::Aux, macros::mul_add};

/// One projected subgradient step (scalar reference): the Discrete Cosine
/// Transform (DCT) data-fidelity gradient plus the Total Variation (TV) and
/// Total Generalized Variation (TGV) gradients, a normalized descent step, and
/// the projection onto the quantized DCT box.
#[allow(clippy::too_many_arguments)]
pub fn solver_step(
    max_rounded_px_w: u32,
    max_rounded_px_h: u32,
    max_rounded_px_count: usize,
    nchannel: usize,
    coefs: &[ScalarCoef],
    auxs: &mut [Aux],
    step_size: f32,
    weight: f32,
    pweight: &[f32; 3],
) {
    for c in 0..nchannel {
        let aux = &mut auxs[c];
        let coef = &coefs[c];

        aux.obj_gradient = AlignedF32::zeros(max_rounded_px_count);

        // DCT coefficient distance
        if pweight[c] != 0.0 {
            dct_gradient(
                max_rounded_px_w,
                max_rounded_px_h,
                pweight[c] * 2.0 * 255.0 * 2.0_f32.sqrt(),
                coef,
                &aux.cos,
                &mut aux.obj_gradient,
            );
        }
    }

    // TV computation
    tv_gradient(max_rounded_px_w, max_rounded_px_h, nchannel, auxs);

    // TGV second order
    tgv_gradient(
        max_rounded_px_w,
        max_rounded_px_h,
        nchannel,
        auxs,
        weight / 2.0_f32.sqrt(),
    );

    // Performs a gradient descent step in the direction of the objective gradient
    // with a specified step size. The gradient is normalized before applying the step.
    for aux in auxs.iter_mut() {
        // Calculate Euclidean norm of the objective gradient
        let norm = aux
            .obj_gradient
            .iter()
            .fold(0.0, |acc, &x| mul_add!(x, x, acc))
            .sqrt();

        // Only update if gradient norm is non-zero
        if norm != 0.0 {
            for i in 0..max_rounded_px_count {
                aux.fdata[i] = step_size.mul_add(-(aux.obj_gradient[i] / norm), aux.fdata[i]);
            }
        }
    }

    // Project onto DCT basis
    auxs.iter_mut().enumerate().for_each(|(c, aux)| {
        project_onto_box(max_rounded_px_w, max_rounded_px_h, aux, &coefs[c]);
    });
}
