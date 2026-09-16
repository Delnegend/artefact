//! CPU-vs-GPU production-solver benchmark.
//!
//! Each case runs the same fixture, production `Artefact` settings, and
//! iteration count on both paths. One `GpuContext` is created outside the timed
//! iterations, so device/adapter selection stays out of the measurement. The
//! timed GPU region still includes per-solve upload, pipeline/bind-group setup,
//! dispatch, and readback because `pipeline::gpu::solve` builds that state for
//! every call.
//!
//! Fixtures:
//! - `smoke`: committed small fixtures at representative iteration checkpoints.
//! - `full`: generated 1600x1200 fixtures at production iterations. Those
//!   generated assets are gitignored, so missing files are skipped, not failed.
//!
//! No timings are asserted; lavapipe can be orders of magnitude slower than a
//! real GPU and drivers/hardware vary.
//!
//! Run with the production pipelines:
//!   RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
//!     cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
//!
//! Selection:
//!   ARTEFACT_BENCH_MATRIX=smoke|full|all (default `all`)
//!   ARTEFACT_BENCH_CASE=case-id-substring (e.g. `sample-422`)
//!   ARTEFACT_BENCH_INPUT=/path/to/image.jpg (replaces the matrix)

use std::{
    hint::black_box,
    path::{Path, PathBuf},
    time::Duration,
};

use artefact_core::{Artefact, JpegSource, ValueCollection, pipeline::gpu::GpuContext};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

#[derive(Clone)]
struct Case {
    id: &'static str,
    path: PathBuf,
    iterations: usize,
}

const SMOKE_CASES: &[(&str, &str, &[usize])] = &[
    ("tiny-420", "tests/fixtures/baseline_420.jpg", &[1, 5]),
    ("tiny-444", "tests/fixtures/baseline_444.jpg", &[1, 5]),
    ("odd-420", "tests/fixtures/odd_420.jpg", &[1, 5]),
];

const FULL_CASES: &[(&str, &str, &[usize])] = &[
    ("sample-420", "../../assets/sample.420.input.jpg", &[50]),
    ("sample-422", "../../assets/sample.422.input.jpg", &[50]),
    ("sample-444", "../../assets/sample.444.input.jpg", &[50]),
    ("sample-j420", "../../assets/sample.j420.input.jpg", &[50]),
    ("sample-j422", "../../assets/sample.j422.input.jpg", &[50]),
    ("sample-j444", "../../assets/sample.j444.input.jpg", &[50]),
];

fn selected_cases() -> Vec<Case> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Ok(path) = std::env::var("ARTEFACT_BENCH_INPUT") {
        return vec![Case {
            id: "custom-input",
            path: PathBuf::from(path),
            iterations: 50,
        }];
    }

    let matrix = std::env::var("ARTEFACT_BENCH_MATRIX").unwrap_or_else(|_| "all".to_string());
    let (want_smoke, want_full) = match matrix.as_str() {
        "smoke" => (true, false),
        "full" => (false, true),
        "all" => (true, true),
        other => {
            eprintln!("unknown ARTEFACT_BENCH_MATRIX={other:?}; using all cases");
            (true, true)
        }
    };
    let filter = std::env::var("ARTEFACT_BENCH_CASE").ok();
    let mut cases = Vec::new();

    for &(id, relative, iterations) in SMOKE_CASES
        .iter()
        .filter(|_| want_smoke)
        .chain(FULL_CASES.iter().filter(|_| want_full))
        .filter(|(id, _, _)| {
            filter
                .as_ref()
                .is_none_or(|want| id.contains(want.as_str()))
        })
    {
        for &iterations in iterations {
            cases.push(Case {
                id,
                path: manifest.join(relative),
                iterations,
            });
        }
    }

    cases
}

fn artefact(path: &Path, iterations: usize) -> Artefact {
    Artefact::default()
        .source(JpegSource::File(path.display().to_string()))
        .benchmark(true)
        .iterations(ValueCollection::ForAll(iterations))
}

pub fn gpu_benches(c: &mut Criterion) {
    let cases: Vec<Case> = selected_cases()
        .into_iter()
        .filter(|case| {
            if case.path.exists() {
                true
            } else {
                eprintln!("skipping {}: missing {}", case.id, case.path.display());
                false
            }
        })
        .collect();
    if cases.is_empty() {
        eprintln!("no benchmark cases selected");
        return;
    }

    let ctx = match pollster::block_on(GpuContext::new()) {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("skipping GPU benchmark: {e}");
            return;
        }
    };
    eprintln!("GPU benchmark adapter: {:?}", ctx.adapter_info());
    eprintln!(
        "benchmark cases: {}",
        cases
            .iter()
            .map(|case| format!("{}@{}", case.id, case.iterations))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut group = c.benchmark_group("process");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(20));

    for case in cases {
        let id = format!("{}-{}", case.id, case.iterations);
        group.bench_with_input(BenchmarkId::new("cpu", &id), &case, |b, case| {
            let artefact = artefact(&case.path, case.iterations);
            b.iter(|| black_box(artefact.process()));
        });
        group.bench_with_input(BenchmarkId::new("gpu", &id), &case, |b, case| {
            let artefact = artefact(&case.path, case.iterations);
            b.iter(|| black_box(pollster::block_on(artefact.process_gpu_with(&ctx))));
        });
    }

    group.finish();
}

criterion_group!(benches, gpu_benches);
criterion_main!(benches);
