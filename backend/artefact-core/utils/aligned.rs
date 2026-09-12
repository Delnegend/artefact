use std::{
    fmt,
    ops::{Deref, DerefMut},
};

/// 64-byte (cache-line) aligned `f32` storage.
///
/// Backed by a `Vec` of 64-byte-aligned chunks, so the allocator hands out a
/// cache-line-aligned buffer and the SIMD kernels' loads/stores cannot split a
/// cache line. Exposes the `[f32]` inside `len` via [`Deref`].
///
/// This is a measurement-driven choice: forcing 64-byte alignment on this
/// machine moved a full 1600x1200 solve ~9% faster than the default 16-byte
/// glibc alignment.
#[repr(align(64))]
#[derive(Clone)]
struct Chunk(#[allow(dead_code)] [f32; 16]);

#[derive(Default)]
pub struct AlignedF32 {
    chunks: Vec<Chunk>,
    len: usize,
}

impl AlignedF32 {
    /// Allocate `len` zeroed elements.
    #[must_use]
    pub fn zeros(len: usize) -> Self {
        Self {
            chunks: vec![Chunk([0.0; 16]); len.div_ceil(16)],
            len,
        }
    }

    const fn as_slice(&self) -> &[f32] {
        // SAFETY: `Chunk` is `#[repr(align(64))]` around `[f32; 16]` (64 bytes,
        // already a multiple of the alignment), so it has the same layout as its
        // 16 `f32`s; the chunks are contiguous and the first `len` elements are
        // initialized.
        unsafe { std::slice::from_raw_parts(self.chunks.as_ptr().cast::<f32>(), self.len) }
    }

    const fn as_mut_slice(&mut self) -> &mut [f32] {
        // SAFETY: as `as_slice`, and `&mut self` guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.chunks.as_mut_ptr().cast::<f32>(), self.len) }
    }
}

impl Deref for AlignedF32 {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for AlignedF32 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl AsRef<[f32]> for AlignedF32 {
    fn as_ref(&self) -> &[f32] {
        self.as_slice()
    }
}

impl AsMut<[f32]> for AlignedF32 {
    fn as_mut(&mut self) -> &mut [f32] {
        self.as_mut_slice()
    }
}

impl Clone for AlignedF32 {
    fn clone(&self) -> Self {
        Self {
            chunks: self.chunks.clone(),
            len: self.len,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.chunks.clone_from(&source.chunks);
        self.len = source.len;
    }
}

impl From<Vec<f32>> for AlignedF32 {
    fn from(value: Vec<f32>) -> Self {
        let mut out = Self::zeros(value.len());
        out.copy_from_slice(&value);
        out
    }
}

impl FromIterator<f32> for AlignedF32 {
    fn from_iter<T: IntoIterator<Item = f32>>(iter: T) -> Self {
        Self::from(iter.into_iter().collect::<Vec<f32>>())
    }
}

impl fmt::Debug for AlignedF32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
