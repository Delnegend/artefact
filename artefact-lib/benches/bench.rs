mod boxing;
mod init_slice;

use criterion::{criterion_group, criterion_main};

criterion_group!(
    benches,
    boxing::boxing_benches,
    init_slice::init_slice_benches
);
criterion_main!(benches);
