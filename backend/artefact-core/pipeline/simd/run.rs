//! Small lane helpers for the TV/TGV kernels. They are inlined unconditionally
//! so the rewritten kernels compile to the same code as the hand-inlined
//! slicing they replaced.

use std::simd::Simd;

use super::traits::{AddSlice, WriteTo};

/// Load exactly `N` lanes starting at `start` (the caller guarantees the run is
/// in-bounds).
#[inline]
pub(super) fn load<const N: usize>(data: &[f32], start: usize) -> Simd<f32, N> {
    data.get(start..start.saturating_add(N))
        .map_or_else(|| Simd::splat(0.0), Simd::from_slice)
}

/// Load `len` (≤ `N`) lanes starting at `start`, zero-filling the tail.
#[inline]
pub(super) fn load_or_zero<const N: usize>(data: &[f32], start: usize, len: usize) -> Simd<f32, N> {
    data.get(start..start.saturating_add(len))
        .map_or_else(|| Simd::splat(0.0), Simd::load_or_default)
}

/// Load `len` lanes at `start` shifted up by one lane: lane `k + 1 = data[k]`,
/// lane 0 = 0.
#[inline]
pub(super) fn load_shifted_right<const N: usize>(
    data: &[f32],
    start: usize,
    len: usize,
) -> Simd<f32, N> {
    load_or_zero::<N>(data, start, len).rotate_elements_right::<1>()
}

/// Add `v`'s lanes `1..` into `target` (i.e. `target[k - 1] += v[k]`), dropping
/// `v`'s lane 0.
#[inline]
pub(super) fn add_shifted_left<const N: usize>(target: &mut [f32], v: Simd<f32, N>) {
    if let Some(end) = N.checked_sub(1) {
        v.add_range_slice(target, 1..=end)
            .write_partial_to(target, 1..=end);
    }
}

/// Mutable run of `len` lanes starting at `start`.
#[inline]
pub(super) fn run_mut(data: &mut [f32], start: usize, len: usize) -> &mut [f32] {
    data.get_mut(start..start.saturating_add(len))
        .unwrap_or(&mut [])
}
