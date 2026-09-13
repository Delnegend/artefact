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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                for in_x in 0..8 {
                    output[index] = input
                        [((block_y * 8 + in_y) * rounded_px_w + (block_x * 8 + in_x)) as usize];

                    index += 1;
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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                let result = f32x8::from_slice(&input[row_start..row_start + 8]);
                output[index..index + 8].copy_from_slice(result.as_array());
                index += 8;
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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                output[index..index + 8].copy_from_slice(&input[row_start..row_start + 8]);
                index += 8;
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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                for in_x in 0..8 {
                    output[((block_y * 8 + in_y) * rounded_px_w + (block_x * 8 + in_x)) as usize] =
                        input[index];

                    index += 1;
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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let result = f32x8::from_slice(&input[index..index + 8]);

                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                output[row_start..row_start + 8].copy_from_slice(result.as_array());
                index += 8;
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

    let mut index = 0;

    for block_y in 0..block_h {
        for block_x in 0..block_w {
            for in_y in 0..8 {
                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                output[row_start..row_start + 8].copy_from_slice(&input[index..index + 8]);
                index += 8;
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
            )
        })
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
            )
        })
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
            )
        })
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
            )
        })
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
            )
        })
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
            )
        })
    });
}
