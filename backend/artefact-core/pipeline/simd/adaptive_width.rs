/// A horizontal run of pixels processed together by the width-generic Total
/// Variation (TV) kernels.
#[derive(Debug, Clone, Copy)]
pub enum AdaptiveWidth {
    X8(u32),
    X16(u32),
    X32(u32),
    X64(u32),
}

/// Calls `$f::<N>(...$arg, run_start, $last)` with the lane width matching `$width`.
///
/// Keeps the direct, monomorphized calls (a `fn`-pointer dispatch loses them)
/// while letting callers pass their argument list only once.
macro_rules! dispatch_run {
    ($width:expr, $f:ident $(, $arg:expr)*; $last:expr) => {
        match $width {
            $crate::pipeline::simd::adaptive_width::AdaptiveWidth::X8(x) => {
                $f::<8>($($arg,)* x, $last)
            }
            $crate::pipeline::simd::adaptive_width::AdaptiveWidth::X16(x) => {
                $f::<16>($($arg,)* x, $last)
            }
            $crate::pipeline::simd::adaptive_width::AdaptiveWidth::X32(x) => {
                $f::<32>($($arg,)* x, $last)
            }
            $crate::pipeline::simd::adaptive_width::AdaptiveWidth::X64(x) => {
                $f::<64>($($arg,)* x, $last)
            }
        }
    };
}
pub(crate) use dispatch_run;

/// Greedy 64 > 32 > 16 > 8 tiling of a row, largest-first.
///
/// Assumes `max_rounded_px_w` is a multiple of 8 (JPEG guarantees this).
#[must_use]
pub fn adaptive_runs(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
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
/// [`adaptive_runs`].
pub fn uniform_runs(max_rounded_px_w: u32) -> Vec<AdaptiveWidth> {
    (0..max_rounded_px_w)
        .step_by(8)
        .map(AdaptiveWidth::X8)
        .collect()
}
