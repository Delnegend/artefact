use std::hint::black_box;

use criterion::Criterion;
use rand::Rng;

fn safe_cast(input: Vec<u16>) -> Vec<f32> {
    input.into_iter().map(|x| x as f32).collect()
}

fn unsafe_cast(input: Vec<u16>) -> Vec<f32> {
    let mut output = Vec::with_capacity(input.len());
    unsafe {
        output.set_len(input.len());
        std::ptr::copy_nonoverlapping(
            input.as_ptr() as *const f32,
            output.as_mut_ptr(),
            input.len(),
        );
    }
    output
}

pub fn casting_benches(c: &mut Criterion) {
    let mut rng = rand::rng();
    let input: Vec<u16> = (0..512 * 512).map(|_| rng.random()).collect();

    let mut group = c.benchmark_group("casting");

    group.bench_function("safe_cast", |b| {
        b.iter(|| safe_cast(black_box(input.clone())))
    });

    group.bench_function("unsafe_cast", |b| {
        b.iter(|| unsafe_cast(black_box(input.clone())))
    });

    group.finish();
}
