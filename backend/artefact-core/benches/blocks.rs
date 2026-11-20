use std::{hint::black_box, simd::f32x8};

use criterion::Criterion;
use rand::RngExt;

fn to_blocks(
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
        return;
    };
    if stride == 0 || bw == 0 || bh == 0 {
        return;
    }

    let mut out_iter = output.iter_mut();
    for in_band in input.chunks_exact(stride.saturating_mul(8)).take(bh) {
        for block_x in 0..bw {
            let col_start = block_x.saturating_mul(8);
            for in_row in in_band.chunks_exact(stride) {
                if let Some(src) = in_row.get(col_start..col_start.saturating_add(8)) {
                    for (&val, dst) in src.iter().zip(&mut out_iter) {
                        *dst = val;
                    }
                }
            }
        }
    }
}

fn to_blocks_simd(
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
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
                    let result = f32x8::from_slice(src);
                    dst.copy_from_slice(result.as_array());
                }
            }
        }
    }
}

fn to_blocks_batch(
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
        return;
    };
    if stride == 0 || bw == 0 || bh == 0 {
        return;
    }

    let mut in_iter = input.iter();
    for out_band in output.chunks_exact_mut(stride.saturating_mul(8)).take(bh) {
        for block_x in 0..bw {
            let col_start = block_x.saturating_mul(8);
            for out_row in out_band.chunks_exact_mut(stride) {
                if let Some(dst) = out_row.get_mut(col_start..col_start.saturating_add(8)) {
                    for (dst_elem, &val) in dst.iter_mut().zip(&mut in_iter) {
                        *dst_elem = val;
                    }
                }
            }
        }
    }
}

pub fn from_blocks_simd(
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
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
                    let result = f32x8::from_slice(src);
                    dst.copy_from_slice(result.as_array());
                }
            }
        }
    }
}

pub fn from_blocks_batch(
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

    let (Ok(stride), Ok(bw), Ok(bh)) = (
        usize::try_from(rounded_px_w),
        usize::try_from(block_w),
        usize::try_from(block_h),
    ) else {
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

pub fn blocks_benches(c: &mut Criterion) {
    let mut rng = rand::rng();
    let input: Vec<f32> = (0..512 * 512).map(|_| rng.random()).collect();
    let mut output = vec![0.0; 512 * 512];

    let mut group = c.benchmark_group("to_blocks");

    group.bench_function("to_blocks", |b| {
        b.iter(|| {
            to_blocks(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });

    group.bench_function("to_blocks_simd", |b| {
        b.iter(|| {
            to_blocks_simd(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });

    group.bench_function("to_blocks_batch", |b| {
        b.iter(|| {
            to_blocks_batch(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });

    group.bench_function("from_blocks", |b| {
        b.iter(|| {
            from_blocks(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });

    group.bench_function("from_blocks_batch", |b| {
        b.iter(|| {
            from_blocks_batch(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });

    group.bench_function("from_blocks_simd", |b| {
        b.iter(|| {
            from_blocks_simd(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            );
        });
    });
}
