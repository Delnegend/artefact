//! Full-solve (`Artefact::process`, benchmark mode) end-to-end timing on the
//! generated 1600x1200 3-channel sample. This is the number tracked across the
//! allocation and cache-alignment work:
//!
//! | revision | median wall |
//! |----------|-------------|
//! | `01118bf` (before alloc/alignment work) | ~1.07 s |
//! | `396277b` (allocations + slice refactor) | ~1.04 s |
//! | `1ef9465` (64-byte aligned `Aux`)        | ~1.00 s |
//!
//! Measurements: `RAYON_NUM_THREADS=8`, AMD Ryzen 9 9950X. Reproduce the
//! historical rows with `git worktree add <dir> <revision>` and run the same
//! command there (they were measured through `artefact-cli`, so criterion's
//! bench profile can read a few percent slower).
//!
//! Run with the production pipeline (`simd`):
//!   RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
//!     cargo bench -p artefact-core --features bench,simd \
//!     --bench bench -- solve
//!
//! Skipped when `assets/sample.420.input.jpg` is absent (`just sample`); set
//! `ARTEFACT_BENCH_INPUT` to override the input path.

use std::{hint::black_box, path::PathBuf, time::Duration};

use artefact_core::{Artefact, JpegSource};
use criterion::Criterion;

fn sample() -> Option<PathBuf> {
    let path = std::env::var("ARTEFACT_BENCH_INPUT").map_or_else(
        |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sample.420.input.jpg"),
        PathBuf::from,
    );
    path.exists().then_some(path)
}

pub fn solve_benches(c: &mut Criterion) {
    let Some(path) = sample() else {
        eprintln!("skipping solve bench: sample asset not found (run `just sample`)");
        return;
    };
    let input = path.to_string_lossy().into_owned();

    let mut group = c.benchmark_group("solve");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(20));
    group.bench_function("process/420", |b| {
        b.iter(|| {
            black_box(
                Artefact::default()
                    .source(JpegSource::File(input.clone()))
                    .benchmark(true)
                    .process(),
            )
        });
    });
    group.finish();
}
