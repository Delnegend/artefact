//! Decode-level tests for the coefficient-extraction path.
//!
//! Fixtures are generated with `cjpeg` from a small synthetic PPM (see the
//! repository history / `scripts/`); they cover the sampling-factor matrix,
//! progressive scans, restart intervals, grayscale and arithmetic coding.

use std::path::PathBuf;

use zune_jpeg::{JpegDecoder, zune_core::bytestream::ZCursor};

#[derive(Debug)]
struct Info {
    width: u16,
    height: u16,
    components: Vec<(u8, u8, u32, u32, usize)>, // (h_factor, v_factor, rounded_w, rounded_h, dct_len)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn decode(bytes: &[u8]) -> Result<Info, String> {
    let mut dec = JpegDecoder::new(ZCursor::new(bytes));
    dec.decode().map_err(|e| format!("{e}"))?;
    let (width, height) = dec.dimensions().ok_or("no dimensions")?;
    let components = dec
        .components
        .iter()
        .map(|c| {
            (
                c.horizontal_samp_factor.u8(),
                c.vertical_samp_factor.u8(),
                c.rounded_px_w,
                c.rounded_px_h,
                c.dct_coefs.len(),
            )
        })
        .collect();
    Ok(Info {
        width,
        height,
        components,
    })
}

fn assert_nonzero_coefficients(bytes: &[u8], name: &str) {
    let mut dec = JpegDecoder::new(ZCursor::new(bytes));
    dec.decode().unwrap_or_else(|e| panic!("{name}: {e}"));
    let total: usize = dec
        .components
        .iter()
        .map(|c| c.dct_coefs.iter().filter(|&&v| v != 0).count())
        .sum();
    assert!(total > 0, "{name}: decoded all-zero coefficients");
}

#[test]
fn baseline_sampling_matrix() {
    // (name, expected per-component (h_factor, v_factor))
    let cases: &[(&str, &[(u8, u8)])] = &[
        ("baseline_444.jpg", &[(1, 1), (1, 1), (1, 1)]),
        ("baseline_422.jpg", &[(1, 1), (2, 1), (2, 1)]),
        ("baseline_420.jpg", &[(1, 1), (2, 2), (2, 2)]),
        // 4:1:1 — luma is 4 wide, chroma 1, so chroma downsample factor is 4
        ("baseline_411.jpg", &[(1, 1), (4, 1), (4, 1)]),
    ];

    for (name, expected) in cases {
        let bytes = fixture(name);
        let info = decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(info.width, 128, "{name}");
        assert_eq!(info.height, 96, "{name}");
        assert_eq!(info.components.len(), 3, "{name}");
        for (i, exp) in expected.iter().enumerate() {
            let (h, v, rw, rh, dct_len) = info.components[i];
            assert_eq!((h, v), *exp, "{name}: component {i} sampling");
            assert!(
                rw % 8 == 0 && rh % 8 == 0,
                "{name}: {rw}x{rh} not 8-aligned"
            );
            assert_eq!(
                dct_len,
                (rw as usize / 8) * (rh as usize / 8) * 64,
                "{name}: component {i} coefficient length"
            );
        }
        assert_nonzero_coefficients(&bytes, name);
    }
}

#[test]
fn progressive_decodes() {
    for name in ["progressive_420.jpg", "progressive_444.jpg"] {
        let bytes = fixture(name);
        let info = decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(info.components.len(), 3, "{name}");
        assert_nonzero_coefficients(&bytes, name);
    }
}

#[test]
fn restart_intervals_decode() {
    for name in ["restart_420.jpg", "progressive_restart_420.jpg"] {
        let bytes = fixture(name);
        decode(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_nonzero_coefficients(&bytes, name);
    }
}

#[test]
fn grayscale_decodes() {
    let bytes = fixture("gray.jpg");
    let info = decode(&bytes).unwrap_or_else(|e| panic!("gray.jpg: {e}"));
    assert_eq!(info.components.len(), 1);
    assert_nonzero_coefficients(&bytes, "gray.jpg");
}

#[test]
fn odd_dimensions_round_to_block_grid() {
    // 17x9, 4:2:0: luma rounded to 32x16, chroma to 16x8
    let bytes = fixture("odd_420.jpg");
    let info = decode(&bytes).expect("odd_420");
    assert_eq!((info.width, info.height), (17, 9));
    let expect: &[(u8, u8, u32, u32)] = &[(1, 1, 32, 16), (2, 2, 16, 8), (2, 2, 16, 8)];
    for (i, (h, v, rw, rh)) in expect.iter().enumerate() {
        let got = info.components[i];
        assert_eq!(
            (got.0, got.1, got.2, got.3),
            (*h, *v, *rw, *rh),
            "odd_420 comp {i}"
        );
    }

    // 17x9, 4:1:1: chroma downsample factor 4
    let bytes = fixture("odd_411.jpg");
    let info = decode(&bytes).expect("odd_411");
    assert_eq!((info.width, info.height), (17, 9));
    let expect: &[(u8, u8, u32, u32)] = &[(1, 1, 32, 16), (4, 1, 8, 16), (4, 1, 8, 16)];
    for (i, (h, v, rw, rh)) in expect.iter().enumerate() {
        let got = info.components[i];
        assert_eq!(
            (got.0, got.1, got.2, got.3),
            (*h, *v, *rw, *rh),
            "odd_411 comp {i}"
        );
    }
}

#[test]
fn arithmetic_is_rejected() {
    let bytes = fixture("arithmetic_420.jpg");
    let err = decode(&bytes).expect_err("arithmetic JPEG should be rejected");
    assert!(
        err.to_lowercase().contains("arithmetic") || err.to_lowercase().contains("unsupported"),
        "unexpected error: {err}"
    );
}
