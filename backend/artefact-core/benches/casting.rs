use std::hint::black_box;

use criterion::Criterion;
use rand::RngExt;

fn safe_cast(input: Vec<u16>) -> Vec<f32> {
    input.into_iter().map(f32::from).collect()
}

fn unsafe_cast(input: &[u16]) -> Vec<f32> {
    let mut output: Vec<f32> = Vec::with_capacity(input.len());
    // SAFETY: `output` has capacity `input.len()`, and `f32` is plain old data;
    // reading `input.len()` `f32`s preserves this legacy benchmark's exact copy semantics.
    unsafe {
        let dst = output.as_mut_ptr().cast::<u16>();
        let src = input.as_ptr();
        std::ptr::copy_nonoverlapping(src, dst, input.len());
        output.set_len(input.len());
    }
    output
}

pub fn casting_benches(c: &mut Criterion) {
    let mut rng = rand::rng();
    let input: Vec<u16> = (0..512 * 512).map(|_| rng.random()).collect();

    let mut group = c.benchmark_group("casting");

    group.bench_function("safe_cast", |b| {
        b.iter(|| safe_cast(black_box(input.clone())));
    });

    group.bench_function("unsafe_cast", |b| {
        b.iter(|| unsafe_cast(black_box(&input)));
    });

    group.finish();
}
