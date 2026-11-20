/// Reorder 8x8 block data (block-major, raster within each block) back into
/// normal raster order.
pub fn from_blocks(
    input: &[f32],
    output: &mut [f32],
    rounded_px_w: u32,
    rounded_px_h: u32,
    block_w: u32,
    block_h: u32,
) {
    debug_assert_eq!(rounded_px_w % 8, 0);
    debug_assert_eq!(rounded_px_h % 8, 0);
    debug_assert_eq!(input.len(), output.len());

    let Ok(stride) = usize::try_from(rounded_px_w) else {
        return;
    };
    let Ok(bw) = usize::try_from(block_w) else {
        return;
    };
    let Ok(bh) = usize::try_from(block_h) else {
        return;
    };
    if stride == 0 || bw == 0 || bh == 0 {
        return;
    }

    let (in_chunks, _) = input.as_chunks::<8>();
    let mut in_iter = in_chunks.iter();
    for out_band in output.chunks_exact_mut(stride.saturating_mul(8)).take(bh) {
        for block_x in 0..bw {
            let col_start = block_x.saturating_mul(8);
            for out_row in out_band.chunks_exact_mut(stride) {
                if let (Some(src), Some(dst)) = (
                    in_iter.next(),
                    out_row.get_mut(col_start..col_start.saturating_add(8)),
                ) {
                    dst.copy_from_slice(src);
                }
            }
        }
    }
}

/// Reorder normal raster data into 8x8 block order (block-major, raster within
/// each block) for per-block transforms.
pub fn to_blocks(
    input: &[f32],
    output: &mut [f32],
    rounded_px_w: u32,
    rounded_px_h: u32,
    block_w: u32,
    block_h: u32,
) {
    debug_assert_eq!(rounded_px_w % 8, 0);
    debug_assert_eq!(rounded_px_h % 8, 0);
    debug_assert_eq!(input.len(), output.len());

    let Ok(stride) = usize::try_from(rounded_px_w) else {
        return;
    };
    let Ok(bw) = usize::try_from(block_w) else {
        return;
    };
    let Ok(bh) = usize::try_from(block_h) else {
        return;
    };
    if stride == 0 || bw == 0 || bh == 0 {
        return;
    }

    let (out_chunks, _) = output.as_chunks_mut::<8>();
    let mut out_iter = out_chunks.iter_mut();
    for in_band in input.chunks_exact(stride.saturating_mul(8)).take(bh) {
        for block_x in 0..bw {
            let col_start = block_x.saturating_mul(8);
            for in_row in in_band.chunks_exact(stride) {
                if let (Some(dst), Some(src)) = (
                    out_iter.next(),
                    in_row.get(col_start..col_start.saturating_add(8)),
                ) {
                    dst.copy_from_slice(src);
                }
            }
        }
    }
}
