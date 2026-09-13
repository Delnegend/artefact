//! Reproducible allocation + wall-time stats for one full solve.
//!
//! Allocation counts are for a full `Artefact::process` (including the RGB
//! output buffer); wall-time is `Artefact::process` in benchmark mode (solver
//! only), matching the CLI's `--benchmark`. Recorded on this machine
//! (`assets/sample.420.input.jpg`, `RAYON_NUM_THREADS=8`, Ryzen 9 9950X):
//!
//! | revision | allocations | bytes allocated | wall median |
//! |----------|-------------|-----------------|-------------|
//! | `01118bf` (before) | 397 | ~1.38e9 | ~1070 ms |
//! | `32b9a12` (allocation churn removed) | 243 | ~2.12e8 | ~1060 ms |
//! | `1ef9465` (64-byte aligned `Aux`) | 243 | ~2.12e8 | ~1000 ms |
//!
//! The allocation reduction is the removal of the per-solver-iteration
//! `obj_gradient` reallocation (`utils/step.rs`); the ~6% wall win is the
//! cache-line-aligned `Aux` buffers (`utils/aligned.rs`). Reproduce historical
//! rows with `git worktree add <dir> <revision>` and run the same command there
//! (the older rows were measured through `artefact-cli`, so wall time can vary
//! a few percent with the profile).
//!
//! Run with the production pipeline (adaptive):
//!   RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
//!     cargo bench -p artefact-core --features simd,simd_adaptive --bench alloc_stats
//!
//! `ARTEFACT_BENCH_INPUT` overrides the input path, `ARTEFACT_BENCH_RUNS` the
//! number of timed runs (default 15). Skipped when the sample asset is absent
//! (`just sample`).

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use artefact_core::{Artefact, JpegSource};

struct Counting;

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn sample() -> Option<PathBuf> {
    let path = std::env::var("ARTEFACT_BENCH_INPUT").map_or_else(
        |_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sample.420.input.jpg"),
        PathBuf::from,
    );
    path.exists().then_some(path)
}

fn build(input: &str, benchmark: bool) -> Artefact {
    Artefact::default()
        .source(JpegSource::File(input.to_string()))
        .benchmark(benchmark)
}

fn solve(input: &str, benchmark: bool) {
    let _ = black_box(build(input, benchmark).process());
}

fn main() {
    let Some(path) = sample() else {
        eprintln!("skipped: sample asset not found (run `just sample`)");
        return;
    };
    let input = path.to_string_lossy().into_owned();
    let runs = std::env::var("ARTEFACT_BENCH_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(15usize);

    // Build the (owned-source) artefact before resetting the counters so the
    // count covers `process` only, not the input-path `String`.
    let artefact = build(&input, false);
    ALLOCS.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    let _ = black_box(artefact.process());
    let allocs = ALLOCS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);

    for _ in 0..2 {
        solve(&input, true);
    }
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            solve(&input, true);
            start.elapsed()
        })
        .collect();
    times.sort_unstable();

    let median = times[times.len() / 2].as_secs_f64() * 1e3;
    let min = times[0].as_secs_f64() * 1e3;
    println!("== full solve: {input}");
    println!("   allocations (full process) = {allocs}");
    println!("   bytes allocated            = {bytes}");
    println!(
        "   wall (benchmark mode)      = {median:.1} ms median, {min:.1} ms min ({runs} runs)"
    );
}
