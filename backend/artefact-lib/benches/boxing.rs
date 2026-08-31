use std::hint::black_box;

use criterion::Criterion;
use rand::Rng;
use wide::f32x8;

fn boxing(
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

fn boxing_simd(
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
                let result = f32x8::from(&input[row_start..row_start + 8]);
                output[index..index + 8].copy_from_slice(result.as_array_ref());
                index += 8;
            }
        }
    }
}

fn boxing_batch(
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

pub fn unboxing(
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

pub fn unboxing_simd(
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
                let result = f32x8::from(&input[index..index + 8]);

                let row_start = ((block_y * 8 + in_y) * rounded_px_w + (block_x * 8)) as usize;
                output[row_start..row_start + 8].copy_from_slice(result.as_array_ref());
                index += 8;
            }
        }
    }
}

pub fn unboxing_batch(
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

pub fn boxing_benches(c: &mut Criterion) {
    let mut rng = rand::rng();
    let input: Vec<f32> = (0..512 * 512).map(|_| rng.random()).collect();
    let mut output = vec![0.0; 512 * 512];

    let mut group = c.benchmark_group("boxing");

    group.bench_function("boxing", |b| {
        b.iter(|| {
            boxing(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            )
        })
    });

    group.bench_function("boxing_simd", |b| {
        b.iter(|| {
            boxing_simd(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            )
        })
    });

    group.bench_function("boxing_batch", |b| {
        b.iter(|| {
            boxing_batch(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            )
        })
    });

    group.bench_function("unboxing", |b| {
        b.iter(|| {
            unboxing(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            )
        })
    });

    group.bench_function("unboxing_batch", |b| {
        b.iter(|| {
            unboxing_batch(
                black_box(&input),
                black_box(&mut output),
                black_box(512),
                black_box(512),
                black_box(64),
                black_box(64),
            )
        })
    });

    group.bench_function("unboxing_simd", |b| {
        b.iter(|| {
            unboxing_simd(
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
