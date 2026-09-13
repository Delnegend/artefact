/// A horizontal run of pixels processed together by the width-generic TV kernels.
#[derive(Debug, Clone, Copy)]
pub enum AdaptiveWidth {
    X8(u32),
    X16(u32),
    X32(u32),
    X64(u32),
}

/// Greedy 64 > 32 > 16 > 8 tiling of a row, largest-first.
///
/// Assumes `max_rounded_px_w` is a multiple of 8 (JPEG guarantees this).
#[must_use]
pub fn get_adaptive_widths(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
    let mut out = Vec::new();
    let mut idx = 0;
    while idx < max_rounded_px_w {
        let remaining = max_rounded_px_w - idx;
        let (width, item) = if remaining >= 64 {
            (64, AdaptiveWidth::X64(idx))
        } else if remaining >= 32 {
            (32, AdaptiveWidth::X32(idx))
        } else if remaining >= 16 {
            (16, AdaptiveWidth::X16(idx))
        } else {
            (8, AdaptiveWidth::X8(idx))
        };
        out.push(item);
        idx += width;
    }
    out
}

/// All-8-wide tiling, used by benches/tests as the fixed-width counterpart to
/// [`get_adaptive_widths`].
pub fn uniform_widths(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
    (0..max_rounded_px_w)
        .step_by(8)
        .map(AdaptiveWidth::X8)
        .collect()
}
