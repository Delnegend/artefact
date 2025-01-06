use criterion::Criterion;
use rand::Rng;
use wide::f32x8;

fn init_with_copy(target: [f32; 64]) -> f32x8 {
    f32x8::from({
        let mut tmp = [0.0; 8];
        tmp[0..=6].copy_from_slice(&target[0..=6]);
        tmp
    })
}

fn manual_init(target: [f32; 64]) -> f32x8 {
    f32x8::new([
        target[0], target[1], target[2], target[3], target[4], target[5], target[6], 0.0,
    ])
}

pub fn init_slice_benches(c: &mut Criterion) {
    let mut target = [0.0; 64];

    for i in 0..64 {
        target[i] = rand::thread_rng().gen();
    }

    let mut group = c.benchmark_group("init_slice");

    group.bench_function("init_with_copy", |b| b.iter(|| init_with_copy(target)));
    group.bench_function("manual_init", |b| b.iter(|| manual_init(target)));
}
