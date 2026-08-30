# Artefact Development Guide

## Directory structure

The project contains 2 main components/directories:

-   [frontend/](./frontend/): the web UI built with Nuxt.js
-   [backend/](./backend/):
    -   [artefact-lib/](./backend/artefact-lib/): the core library that does the image processing
    -   [artefact-cli/](./backend/artefact-cli/): the command-line interface that uses artefact-lib
    -   [artefact-wasm/](./backend/artefact-wasm/): the WebAssembly bindings for artefact-lib, to be used in the web UI
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

The [justfile](../justfile) includes a recipe to generate subsampled JPEG test images using `ffmpeg`. Because `ffmpeg` is relatively large and typically only needed once, it is not listed as a prerequisite or installed in the devcontainer.

### Option 1: Using `ffmpeg` on the host

If you have `ffmpeg` and `just` installed on your host, place a test image named `sample.png` in the `assets/` directory and run:

```bash
just encode
```

The generated images will be written to the `assets/` directory.

### Option 2: Using `ffmpeg` inside the devcontainer

If you want to contain everything inside the devcontainer, you can install `ffmpeg` there.

```bash
sudo apt update && sudo apt install -y ffmpeg
```

After installation, place `sample.png` in `assets/` and run:

```bash
just encode
```

## Running against a sample image

```bash
just decode <chroma-subsampling>
```

Where `<chroma-subsampling>` is one of: `420`, `422`, `444`, `j420`, `j422`, `j444`.

## Cross-compiling / Releases

Cross-compiled CLI binaries (linux `x86_64-unknown-linux-musl`, windows `x86_64-pc-windows-gnu`, macOS `aarch64-apple-darwin` for Apple Silicon) are built on GitHub Actions via `.github/workflows/release.yml`.

Trigger via `workflow_dispatch` with inputs `release_version` (e.g. `0.1.0`) and `create_release` (`true` to push to `Releases`), or automatically on merged `pull_request` to `next`/`main` with `dependencies` label or `chore/update-deps` branch (auto-bumps patch).

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

To toggle specific SIMD features when building the CLI, modify [artefact-cli's Cargo.toml](./backend/artefact-cli/Cargo.toml) and add the desired features to the `[dependencies.artefact-lib]` features list.

Example:

```toml
[dependencies.artefact-lib]
path = "../artefact-lib"
features = [
"simd", # enable SIMD
"simd_std", # using `std::simd` instead of `wide`
"simd_adaptive", # dynamically switch between x8, x16, x32 and x64
"native", # use LLVM "mul_add" intrinsic for more accurate rounding, requires "-Ctarget-cpu=native" or else it'll most likely be slower
"moz", # use `mozjpeg` instead of `zune-jpeg` for decoding, might provide better compatibility
]
```

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
