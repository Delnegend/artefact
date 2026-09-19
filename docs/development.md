# Artefact Development Guide

## Directory structure

The project contains 2 main components/directories:

-   [frontend/](./frontend/): the web UI built with Nuxt.js
-   [backend/](./backend/):
    -   [artefact-core/](./backend/artefact-core/): the core library that does the image processing
    -   [artefact-cli/](./backend/artefact-cli/): the command-line interface that uses artefact-core
    -   [artefact-wasm/](./backend/artefact-wasm/): the WebAssembly bindings for artefact-core, to be used in the web UI
    -   [zune-jpeg/](./backend/zune-jpeg/): a fork of zune-jpeg with some fixes, improvements, and DCT coefficients exposed

## Prerequisites

These will be automatically installed if you choose to run this project inside a devcontainer:

-   [the Rust toolchain (with `rustup`)](https://rust-lang.org/learn/get-started/).
-   [`just`](https://github.com/casey/just).
-   [`bun`](https://bun.sh/) for building and running the frontend.
-   [`wasm-pack`](https://github.com/rustwasm/wasm-pack) to build the WASM library version of artefact.
-   `zip`, `tar` only if manually archiving (releases for linux x64, windows x64, macOS arm64 are built on GitHub Actions via `.github/workflows/release.yml`).

<!-- -   `cargo-flamegraph`, `perf` are optional, used for performance profiling. -->

## Sample images

Generate subsampled test inputs from `assets/sample.png` (requires `ffmpeg`, not in devcontainer):

```bash
sudo apt-get update && sudo apt-get install ffmpeg
just encode  # → assets/sample.{j444,j422,j420,444,422,420}.input.jpg (6 files)
```

Place `assets/sample.png` first; outputs go to `assets/`.

## Test a sample

Decode a generated sample:

```bash
just decode 420  # reads assets/sample.420.input.jpg → assets/sample.420.decoded.png
```

`420` may be `420`/`422`/`444`/`j420`/`j422`/`j444`; see `just --choose`.

## Cross-compiling / Releases

Built on GitHub Actions (`.github/workflows/release.yml`) for `linux x64`, `windows x64`, `macOS arm64`.

Trigger: `workflow_dispatch` (`release_version`, `create_release`) or auto on `PR → main` with `dependencies` label (patch bump).

Locally, just build natively:

```bash
just build
# or
cargo build --bin artefact-cli --release
# manual cross (if toolchain installed):
# cargo build --bin artefact-cli --release --target x86_64-unknown-linux-gnu
# cargo build --bin artefact-cli --release --target x86_64-pc-windows-gnu
# cargo build --bin artefact-cli --release --target aarch64-apple-darwin
```

## Solver pipelines

- CPU scalar (`pipeline::scalar`) is the frozen reference.
- CPU SIMD (`pipeline::simd`) is the production default. Native uses adaptive x8/x16/x32/x64 dispatch; wasm32 uses uniform x8 because wider `std::simd` vectors miscompiled there.
- GPU (`pipeline::gpu`, optional `gpu` feature) solves with one storage buffer per channel, `aux` scratch, `meta` metadata, and uniform `params`. It uses gather-only compute kernels, two submits per iteration, and reusable `GpuContext`; `process_gpu_with` allows repeated solves without recreating the device/queue.
- Feature selection: `simd` selects the CPU SIMD pipeline (or scalar without it); `gpu` adds `process_gpu`, `process_gpu_with`, and `process_auto`, which falls back to CPU when no adapter/solver is available.

[artefact-cli's Cargo.toml](./backend/artefact-cli/Cargo.toml) and [artefact-wasm's Cargo.toml](./backend/artefact-wasm/Cargo.toml) enable `simd,gpu`, so both shipped binaries can use SIMD and the GPU backend. Enable them manually for ad-hoc builds with `--features simd,gpu`.

```toml
[dependencies.artefact-core]
path = "../artefact-core"
features = ["simd"] # adaptive x8/x16/x32/x64 dispatch via `std::simd`
```

## Sample images & regression

`scripts/generate-sample.sh` builds the synthetic `assets/sample.png` (1600×1200, gradients/color blocks/patterns/text) and encodes all 6 chroma-subsampled JPGs (`j444/j422/j420/444/422/420`). Decoding regressions are covered by native Rust tests (`cargo test --workspace`, run as part of `just check`): `backend/zune-jpeg/tests/decode.rs` decodes committed `cjpeg` fixtures (4:4:4/4:2:2/4:2:0/4:1:1, progressive, restart intervals, grayscale, arithmetic-rejected) and `backend/artefact-core/tests/verify.rs` checks reconstructed color blocks end-to-end. `just sample` regenerates the large sample inputs. `just check` sets `ARTEFACT_REQUIRE_GPU=1` so GPU smoke/equivalence tests and the committed `odd_420.jpg` edge case must really run there; plain `cargo test` still skips gracefully when no adapter is present.

## Benchmarks

Full-solve timing and allocation stats live in `backend/artefact-core/benches/` and smoke cases use committed fixtures; full 1600x1200 cases require the generated sample (`just sample`):

```bash
# criterion: end-to-end CPU solver time (assets/sample.420.input.jpg)
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd --bench bench -- solve

# CPU-vs-GPU production solves over smoke and/or full fixtures
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
```

Select the benchmark matrix without editing code:

```bash
ARTEFACT_BENCH_MATRIX=smoke cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
ARTEFACT_BENCH_MATRIX=full ARTEFACT_BENCH_CASE=sample-422 cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
ARTEFACT_BENCH_INPUT=/tmp/input.jpg cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
```

Benchmarks do not assert timings. Reused `GpuContext` setup stays outside the measurement, while each GPU measurement includes decode, upload, pipeline/bind-group setup, dispatch, readback, and finalization. CI’s lavapipe driver is correctness coverage only. Record fixture, chroma, iterations, CPU threads, compiler flags, adapter/backend/driver with any reported number.

### Recorded CPU vs GPU numbers

`benches/gpu.rs`, 1600x1200 sampled fixtures, 50 iterations, production settings, `RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8`, AMD Radeon (RADV GFX1200, discrete, Mesa 25.0.7), criterion medians:

| Case | CPU | GPU | Speedup |
|------|-----|-----|---------|
| `sample-420`  | 962 ms  | 213 ms | 4.5× |
| `sample-422`  | 1015 ms | 220 ms | 4.6× |
| `sample-444`  | 1013 ms | 224 ms | 4.5× |
| `sample-j420` | 963 ms  | 213 ms | 4.5× |
| `sample-j422` | 1015 ms | 219 ms | 4.6× |
| `sample-j444` | 1016 ms | 224 ms | 4.5× |

The committed `smoke` fixtures are far below the GPU's fixed dispatch/readback cost (e.g. `tiny-420@1`: CPU ~164 µs vs GPU ~3.0 ms), which is why the harness keeps them for correctness/smoke, not speed comparisons. The equivalent CLI run (`artefact-cli --gpu --benchmark`) is ~0.28-0.32 s because it also pays process start and context creation.

```bash
# allocations + wall time for one solve
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features simd --bench alloc_stats
```

`benches/solve.rs` and `benches/alloc_stats.rs` document the recorded before/after numbers (allocation churn removal in `32b9a12`, 64-byte-aligned `Aux` buffers in `1ef9465`); reproduce older revisions with `git worktree add <dir> <revision>`. Keep `RAYON_NUM_THREADS` fixed when comparing.

## Building the WASM library and web UI

Build the WASM library if it has not already been built or if there are changes.

```bash
just build wasm
```

Then build the frontend.

```bash
just build web
```

The async wasm `compute` API takes `use_gpu`; the worker probes for a usable WebGPU adapter (including `GPUAdapter.info`) and requests CPU otherwise. PWA precache includes `wasm`.

## Other recipes

Check out the [justfile](../justfile) for other available recipes for development, building, testing, linting, etc.
