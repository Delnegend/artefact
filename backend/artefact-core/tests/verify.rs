//! End-to-end decode regression test (native replacement for `scripts/verify.sh`).
//!
//! Decodes small committed fixtures through the public `Artefact` solver and
//! checks the reconstructed pixels. Fixtures are `cjpeg` encodings of a
//! synthetic image with four saturated 32x32 blocks:
//!   red (16..48,16..48), green (64..96,16..48),
//!   blue (16..48,56..88), yellow (64..96,56..88)
//! on a gradient background, at 128x96.

use std::path::PathBuf;

use artefact_core::{Artefact, JpegSource};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn process(name: &str) -> artefact_core::image::RgbImage {
    let path = fixture(name);
    Artefact::default()
        .source(JpegSource::File(path.display().to_string()))
        .process()
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn channel_dist(a: (u8, u8, u8), b: (u8, u8, u8)) -> u32 {
    (i32::from(a.0) - i32::from(b.0))
        .unsigned_abs()
        .max((i32::from(a.1) - i32::from(b.1)).unsigned_abs())
        .max((i32::from(a.2) - i32::from(b.2)).unsigned_abs())
}

#[test]
fn reconstructs_color_blocks() {
    // (fixture, tolerance) — higher tolerance for the more subsampled chroma
    let cases = [
        ("baseline_444.jpg", 40u32),
        ("baseline_422.jpg", 50),
        ("baseline_420.jpg", 60),
        ("baseline_411.jpg", 70),
        ("progressive_420.jpg", 60),
        ("restart_420.jpg", 60),
        ("progressive_restart_420.jpg", 60),
    ];

    for (name, tol) in cases {
        let img = process(name);
        assert_eq!(img.dimensions(), (128, 96), "{name}: dimensions");

        let expect = [
            ("red", (32, 32), (255, 0, 0)),
            ("green", (80, 32), (0, 255, 0)),
            ("blue", (32, 72), (0, 0, 255)),
            ("yellow", (80, 72), (255, 255, 0)),
        ];
        for (label, (x, y), rgb) in expect {
            let px = img.get_pixel(x, y);
            let got = (px[0], px[1], px[2]);
            let d = channel_dist(got, rgb);
            assert!(
                d <= tol,
                "{name}: {label} block at ({x},{y}) = {got:?}, expected ~{rgb:?} (max channel diff {d} > {tol})"
            );
        }
    }
}

#[test]
fn grayscale_stays_gray() {
    let img = process("gray.jpg");
    assert_eq!(img.dimensions(), (128, 96));
    // No chroma: R, G and B must track each other closely.
    for (x, y) in [(10, 10), (32, 32), (80, 72), (120, 90)] {
        let px = img.get_pixel(x, y);
        let spread = px.0.iter().max().unwrap() - px.0.iter().min().unwrap();
        assert!(spread <= 4, "gray.jpg: ({x},{y}) spread {spread} in {px:?}");
    }
}

#[test]
fn corrupt_input_is_an_error_not_a_panic() {
    // Truncated baseline: must return an error rather than panic/garbage.
    let bytes = std::fs::read(fixture("baseline_420.jpg")).unwrap();
    let truncated = bytes[..bytes.len() / 2].to_vec();
    let result = Artefact::default()
        .source(JpegSource::Buffer(truncated))
        .iterations(artefact_core::ValueCollection::ForAll(1))
        .process();
    assert!(result.is_err(), "truncated JPEG should fail to decode");
}

/// One-off dev harness: with the solver disabled (`iterations = 0`), our
/// dequant + IDCT + YCbCr->RGB must reproduce libjpeg's (`djpeg`) decode of the
/// same file, which validates the vendored `zune-jpeg` coefficient extraction
/// against a reference implementation. Only meaningful for full-resolution
/// components (4:4:4 / grayscale) where upsampling does not differ.
///
/// Skipped when `djpeg` is not installed (e.g. CI). Only compiled for the
/// production `simd` pipeline: the scalar reference's `From` scrambles
/// its init (see `pipeline/tests.rs`), so it cannot be compared directly.
#[cfg(feature = "simd")]
#[test]
fn matches_libjpeg_reference_when_available() {
    use std::process::Command;

    if Command::new("djpeg").arg("-version").output().is_err() {
        eprintln!("skipping: djpeg not found");
        return;
    }

    let cases: &[(&str, &[&str])] = &[
        ("baseline_444.jpg", &[]),
        ("progressive_444.jpg", &[]),
        ("gray.jpg", &["-grayscale"]),
    ];

    for (name, extra) in cases {
        let ours = Artefact::default()
            .source(JpegSource::File(fixture(name).display().to_string()))
            .weight(artefact_core::ValueCollection::ForAll(0.0))
            .pweight(artefact_core::ValueCollection::ForAll(0.0))
            .iterations(artefact_core::ValueCollection::ForAll(0))
            .process()
            .unwrap_or_else(|e| panic!("{name}: {e}"));

        let ref_path = std::env::temp_dir().join(format!("artefact-ref-{name}.pnm"));
        let status = Command::new("djpeg")
            .args(*extra)
            .arg("-outfile")
            .arg(&ref_path)
            .arg(fixture(name))
            .status()
            .expect("run djpeg");
        assert!(status.success(), "{name}: djpeg failed");

        let reference = read_pnm(&std::fs::read(&ref_path).expect("read reference"));
        let _ = std::fs::remove_file(&ref_path);

        assert_eq!(
            (ours.width() as usize, ours.height() as usize),
            (reference.0, reference.1),
            "{name}: dimensions"
        );

        let mut max_diff = 0u8;
        for (a, b) in ours.pixels().zip(reference.2.iter()) {
            for c in 0..3 {
                max_diff = max_diff.max(a[c].abs_diff(b[c]));
            }
        }
        // float IDCT vs libjpeg's integer IDCT: a couple of LSBs is expected.
        assert!(
            max_diff <= 4,
            "{name}: diverges from libjpeg by {max_diff} (> 4)"
        );
        println!("{name}: max diff vs djpeg = {max_diff}");
    }
}

/// Minimal P5/P6 reader (djpeg output), avoiding an extra `image` format.
fn read_pnm(data: &[u8]) -> (usize, usize, Vec<[u8; 3]>) {
    let mut pos = 0;
    let mut token = || {
        while data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if data[pos] == b'#' {
            while data[pos] != b'\n' {
                pos += 1;
            }
            while data[pos].is_ascii_whitespace() {
                pos += 1;
            }
        }
        let start = pos;
        while !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        std::str::from_utf8(&data[start..pos]).unwrap().to_string()
    };

    let magic = token();
    let w: usize = token().parse().unwrap();
    let h: usize = token().parse().unwrap();
    let _max = token();
    pos += 1; // single whitespace after maxval

    let mut out = Vec::with_capacity(w * h);
    if magic == "P5" {
        for &g in &data[pos..pos + w * h] {
            out.push([g, g, g]);
        }
    } else {
        for px in data[pos..pos + w * h * 3].chunks_exact(3) {
            out.push([px[0], px[1], px[2]]);
        }
    }
    (w, h, out)
}
