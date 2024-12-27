mod boxing;

use criterion::{criterion_group, criterion_main};

criterion_group!(benches, boxing::boxing_benches);
criterion_main!(benches);
