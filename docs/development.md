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
-   `zip`, `tar` only if manually archiving (releases for linux x64 musl, windows x64, macOS arm64 are built on GitHub Actions via `.github/workflows/release.yml`).

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

Built on GitHub Actions (`.github/workflows/release.yml`) for `linux x64 musl`, `windows x64`, `macOS arm64`.

Trigger: `workflow_dispatch` (`release_version`, `create_release`) or auto on `PR → main` with `dependencies` label (patch bump).

Locally, just build natively:

```bash
just build
# or
cargo build --bin artefact-cli --release
# manual cross (if toolchain installed):
# cargo build --bin artefact-cli --release --target x86_64-unknown-linux-musl
# cargo build --bin artefact-cli --release --target x86_64-pc-windows-gnu
# cargo build --bin artefact-cli --release --target aarch64-apple-darwin
```

## SIMD implementation

Pipelines live in `backend/artefact-core/pipeline/{scalar,simd8,adaptive}` — `scalar` is the frozen reference, `adaptive` is the production default (`simd,simd_adaptive` features), `simd8` is a fixed-width reference. Shared logic (FISTA, projection, step orchestration, DCT, boxing, SIMD traits) lives in `backend/artefact-core/utils/`. `std::simd` is used everywhere (no `wide`); `scalar` keeps its own scalar loops so it can be diffed against the SIMD paths.

To toggle specific SIMD features when building the CLI, modify [artefact-cli's Cargo.toml](./backend/artefact-cli/Cargo.toml) and add the desired features to the `[dependencies.artefact-core]` features list.

Example:

```toml
[dependencies.artefact-core]
path = "../artefact-core"
features = [
"simd", # enable SIMD via `std::simd`
"simd_adaptive", # dynamically switch between x8, x16, x32 and x64
"native", # use LLVM "mul_add" intrinsic for more accurate rounding, requires "-Ctarget-cpu=native" or else it'll most likely be slower
]
```

## Sample images & regression

`scripts/generate-sample.sh` builds the synthetic `assets/sample.png` (1600×1200, gradients/color blocks/patterns/text) and encodes all 6 chroma-subsampled JPGs (`j444/j422/j420/444/422/420`). Decoding regressions are covered by native Rust tests (`cargo test --workspace`, run as part of `just check`): `backend/zune-jpeg/tests/decode.rs` decodes committed `cjpeg` fixtures (4:4:4/4:2:2/4:2:0/4:1:1, progressive, restart intervals, grayscale, arithmetic-rejected) and `backend/artefact-core/tests/verify.rs` checks reconstructed color blocks end-to-end. `just sample` regenerates the large sample inputs.

## Benchmarks

Full-solve timing and allocation stats live in `backend/artefact-core/benches/` and require the generated sample (`just sample`):

```bash
# criterion: end-to-end solver time (assets/sample.420.input.jpg)
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd,simd_adaptive --bench bench -- solve

# allocations + wall time for one solve
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features simd,simd_adaptive --bench alloc_stats
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

## Other recipes

Check out the [justfile](../justfile) for other available recipes for development, building, testing, linting, etc.
