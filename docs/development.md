# Artefact Development Guide

> See also [architecture.md](architecture.md) (layout, solver pipelines) and
> [cli.md](cli.md) (flags, tuning).

## Prerequisites

These are automatically installed if you run the project inside a devcontainer:

```bash
# VS Code: Command Palette → Reopen in Container
# CLI:
devcontainer up --workspace-folder .
```

The image bakes in Rust `nightly` + `rust-analyzer`, `mold` 2.42.1,
`cargo-binstall`/`flamegraph`/`wasm-pack`, `just`, `fzf`, `bun`, and `node`, so the toolchain works
for editors that skip `postCreateCommand` (e.g. Zed). `postinstall.sh` only runs `bun i` in
`frontend`.

Without the devcontainer:

- [Rust](https://www.rust-lang.org) via `rustup` (`nightly`, `minimal` profile)
- [`just`](https://github.com/casey/just), [`bun`](https://bun.sh),
  [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
- `ffmpeg` only for sample image generation (not in the devcontainer by default)
- `zip`/`tar` only if manually archiving — releases (linux x64, windows x64, macOS arm64) are
  built on GitHub Actions via `.github/workflows/release.yml`

## Build

```bash
# native CLI (release, LTO) -> target/release/artefact-cli
just build
# or: cargo build --bin artefact-cli --release

# WASM lib (generates frontend/src/utils/artefact-wasm)
just build wasm
# or: wasm-pack build backend/artefact-wasm --target web --out-dir frontend/src/utils/artefact-wasm

# web (static build for GitHub Pages -> frontend/dist)
just build web
# or: cd frontend && bun x vite build

# frontend dev server (hot reload)
just dev
# or: cd frontend && bun x vite
```

## Checks

```bash
just check          # fmt + clippy + tests + oxlint + oxfmt (all)
just check rust     # Rust only
just check js       # frontend only (oxlint + oxfmt + vue-tsc)
```

Formatting is applied, not gated: `cargo fmt` for Rust, `oxfmt` for JS. The gate is oxlint +
`vue-tsc` + clippy + tests.

`just check rust` sets `ARTEFACT_REQUIRE_GPU=1`, so GPU smoke/equivalence tests and the committed
`odd_420.jpg` edge case cannot pass by skipping a missing adapter. Plain `cargo test` keeps the
lenient skips for machines without a GPU.

## Sample images and profiling

```bash
just sample         # scripts/generate-sample.sh (needs ffmpeg) -> assets/sample.png
just flame 420      # flamegraph for assets/sample.420.input.jpg
```

`scripts/generate-sample.sh` builds the synthetic `assets/sample.png` (1600×1200, gradients, color
blocks, patterns, text) and encodes all 6 chroma-subsampled JPGs
(`j444`/`j422`/`j420`/`444`/`422`/`420`). Sample fixtures are committed, so tests and benches only
need this when you want a fresh sample.

`just flame <chroma>` accepts `420`/`422`/`444`/`j420`/`j422`/`j444`; see `just --choose` for the
recipe list. To decode one by hand, run `artefact-cli assets/sample.420.input.jpg -y` directly.

## Sample images & regression

Decoding regressions are covered by native Rust tests (`cargo test --workspace`, run as part of
`just check`): `backend/zune-jpeg/tests/decode.rs` decodes committed `cjpeg` fixtures (4:4:4,
4:2:2, 4:2:0, 4:1:1, progressive, restart intervals, grayscale, arithmetic-rejected) and
`backend/artefact-core/tests/verify.rs` checks reconstructed color blocks end-to-end.
`backend/artefact-core/pipeline/tests.rs` exercises each pipeline against the sample fixtures and
skips gracefully when they are absent.

## Benchmarks

Full-solve timing and allocation stats live in `backend/artefact-core/benches/` and smoke cases use
committed fixtures; full 1600x1200 cases require the generated sample (`just sample`):

```bash
# criterion: end-to-end CPU solver time (assets/sample.420.input.jpg)
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd --bench bench -- solve

# CPU-vs-GPU production solves over smoke and/or full fixtures
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd,gpu --bench gpu

# allocations + wall time for one solve
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features simd --bench alloc_stats
```

Select the benchmark matrix without editing code:

```bash
ARTEFACT_BENCH_MATRIX=smoke cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
ARTEFACT_BENCH_MATRIX=full ARTEFACT_BENCH_CASE=sample-422 cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
ARTEFACT_BENCH_INPUT=/tmp/input.jpg cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
```

Benchmarks do not assert timings. Reused `GpuContext` setup stays outside the measurement, while
each GPU measurement includes decode, upload, pipeline/bind-group setup, dispatch, readback, and
finalization. CI's lavapipe driver is correctness coverage only. Record fixture, chroma, iterations,
CPU threads, compiler flags, and adapter/backend/driver with any reported number.

### Recorded CPU vs GPU numbers

`benches/gpu.rs`, 1600x1200 sampled fixtures, 50 iterations, production settings,
`RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8`, AMD Radeon (RADV GFX1200, discrete, Mesa
25.0.7), criterion medians:

| Case | CPU | GPU | Speedup |
|------|-----|-----|---------|
| `sample-420`  | 962 ms  | 213 ms | 4.5× |
| `sample-422`  | 1015 ms | 220 ms | 4.6× |
| `sample-444`  | 1013 ms | 224 ms | 4.5× |
| `sample-j420` | 963 ms  | 213 ms | 4.5× |
| `sample-j422` | 1015 ms | 219 ms | 4.6× |
| `sample-j444` | 1016 ms | 224 ms | 4.5× |

The committed `smoke` fixtures are far below the GPU's fixed dispatch/readback cost (e.g.
`tiny-420@1`: CPU ~164 µs vs GPU ~3.0 ms), which is why the harness keeps them for
correctness/smoke, not speed comparisons. The equivalent CLI run (`artefact-cli --gpu
--benchmark`) is ~0.28-0.32 s because it also pays process start and context creation.

`benches/solve.rs` and `benches/alloc_stats.rs` document the recorded before/after numbers
(allocation churn removal in `32b9a12`, 64-byte-aligned `Aux` buffers in `1ef9465`); reproduce older
revisions with `git worktree add <dir> <revision>`. Keep `RAYON_NUM_THREADS` fixed when comparing.

## Cross-compiling / releases

Built on GitHub Actions (`.github/workflows/release.yml`) for `linux x64`, `windows x64`,
`macOS arm64`.

Trigger: weekly cron (Sunday 00:00 UTC) or `workflow_dispatch`. Version comes from Conventional
Commits since the latest tag via `ietf-tools/semver-action`.

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

## Other recipes

Check out the [justfile](../justfile) for the full recipe list (`just --choose`).
