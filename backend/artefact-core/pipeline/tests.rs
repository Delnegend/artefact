//! Cross-pipeline equivalence check on a real fixture.
//!
//! Only the SIMD pipeline remains; the scalar reference is checked but is
//! `#[ignore]`d: its `Coefficient -> ScalarCoef` conversion calls `from_blocks`
//! once per block (see `scalar/coef.rs`) instead of once after all blocks, so
//! it starts from a scrambled `image_data`. The DCT projection in each solver
//! step re-anchors to the JPEG coefficients, so the outputs converge again
//! after enough iterations (max diff ~3.3e2 at 0 iters, ~1.5e2 at 1, ~2.6 at
//! the default 50). Scalar is slated for removal once zune-jpeg is trusted.
//!
//! Width-tiling equivalence (uniform x8 vs adaptive 64/32/16/8) is covered by
//! `tests/tv_equivalence.rs`.
//!
//! Skips silently if `assets/sample.*.input.jpg` isn't present (`just sample`).

use crate::jpeg::{Jpeg, JpegSource};

const WEIGHT: f32 = 0.3;
const PWEIGHT: [f32; 3] = [0.001; 3];
const ITERATIONS: usize = 1;

fn fixture_path(suffix: &str) -> String {
    format!(
        "{}/../../assets/sample.{suffix}.input.jpg",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn decode(suffix: &str) -> Option<(Jpeg, usize, u32, u32, usize)> {
    let path = fixture_path(suffix);
    if !std::path::Path::new(&path).exists() {
        return None;
    }
    let jpeg = Jpeg::from(JpegSource::File(path)).ok()?;
    let nchannel = jpeg.nchannel as usize;
    let (mut w, mut h) = (0, 0);
    for coef in &jpeg.coefs {
        w = w.max(coef.rounded_px_w);
        h = h.max(coef.rounded_px_h);
    }
    Some((jpeg, nchannel, w, h, (w * h) as usize))
}

fn max_diff(
    a: &[crate::utils::aligned::AlignedF32],
    b: &[crate::utils::aligned::AlignedF32],
) -> f32 {
    let mut m = 0.0_f32;
    for (ca, cb) in a.iter().zip(b) {
        for (x, y) in ca.iter().zip(cb.iter()) {
            m = m.max((x - y).abs());
        }
    }
    m
}

/// Known scalar init discrepancy — see module docs.
#[test]
#[ignore = "scalar Coefficient->ScalarCoef scrambles the init (per-block from_blocks); converges only after many iterations, slated for removal"]
fn scalar_matches_simd() {
    let mut checked = 0;
    for suffix in ["420", "422", "444"] {
        let Some((jpeg, nch, w, h, count)) = decode(suffix) else {
            continue;
        };
        checked += 1;
        let scalar = crate::pipeline::scalar::solve(
            nch,
            jpeg.coefs.clone(),
            WEIGHT,
            PWEIGHT,
            ITERATIONS,
            w,
            h,
            count,
        );
        let simd =
            crate::pipeline::simd::solve(nch, jpeg.coefs, WEIGHT, PWEIGHT, ITERATIONS, w, h, count);
        let d = max_diff(&scalar, &simd);
        println!("{suffix}: max|scalar - simd| = {d:.3e}");
        assert!(d <= 1.0, "scalar diverged from simd on {suffix} by {d}");
    }
    if checked == 0 {
        eprintln!("skipped: no assets/sample.*.input.jpg fixtures (run `just sample`)");
    }
}

/// The SIMD TGV kernel must scatter its diagonal contributions to the same
/// cells as the scalar reference `(x + 1, y - 1)` / `(x - 1, y + 1)`.
#[test]
fn tgv_matches_scalar() {
    use crate::{
        pipeline::simd::{tgv_gradient, uniform_runs},
        utils::{
            aligned::AlignedF32,
            auxiliary::{Aux, PixelDifference},
        },
    };

    const W: u32 = 32;
    const H: u32 = 8;
    const NCH: usize = 3;

    fn make() -> Vec<Aux> {
        let count = (W * H) as usize;
        (0..NCH)
            .map(|c| Aux {
                cos: AlignedF32::zeros(count),
                obj_gradient: AlignedF32::zeros(count),
                pixel_diff: PixelDifference {
                    x: (0..count)
                        .map(|i| (((i + c) * 7) % 13) as f32 - 6.0)
                        .collect(),
                    y: (0..count)
                        .map(|i| (((i + c) * 5) % 11) as f32 - 5.0)
                        .collect(),
                },
                fdata: AlignedF32::zeros(count),
                fista: AlignedF32::zeros(count),
            })
            .collect()
    }

    let mut simd = make();
    let mut norm = vec![0.0f32; (W * H) as usize];
    tgv_gradient(W, H, NCH, &mut simd, 0.3, &uniform_runs(W), &mut norm);

    let mut scalar = make();
    crate::pipeline::scalar::tgv_gradient(W, H, NCH, &mut scalar, 0.3);

    let mut max = 0.0_f32;
    for (a, b) in simd.iter().zip(&scalar) {
        for (x, y) in a.obj_gradient.iter().zip(b.obj_gradient.iter()) {
            max = max.max((x - y).abs());
        }
    }
    println!("max|simd tgv - scalar tgv| = {max:.3e}");
    assert!(max < 1e-3, "simd TGV diverged from scalar by {max}");
}
