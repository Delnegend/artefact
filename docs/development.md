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
-   [`just`](https://github.com/casey/just), and set `alias j=just` for convenience.
-   [`bun`](https://bun.sh/) for building and running the frontend.
-   [`wasm-pack`](https://github.com/rustwasm/wasm-pack) to build the WASM library version of artefact.
-   mingw toolchains (`gcc-mingw-w64-*`) and musl targets, required for cross-compiling to Windows and static Linux binaries.
-   `zip`, `tar` for packaging release archives.

<!-- -   `cargo-flamegraph`, `perf` are optional, used for performance profiling. -->

## Sample images

The [justfile](../justfile) includes a recipe to generate subsampled JPEG test images using `ffmpeg`. Because `ffmpeg` is relatively large and typically only needed once, it is not listed as a prerequisite or installed in the devcontainer.

### Option 1: Using `ffmpeg` on the host

If you have `ffmpeg` and `just` installed on your host, place a test image named `sample.png` in the `assets/` directory and run:

```bash
j encode
```

The generated images will be written to the `assets/` directory.

### Option 2: Using `ffmpeg` inside the devcontainer

If you want to contain everything inside the devcontainer, you can install `ffmpeg` there.

```bash
sudo apt update && sudo apt install -y ffmpeg
```

After installation, place `sample.png` in `assets/` and run:

```bash
j encode
```

## Running against a sample image

```bash
j decode <chroma-subsampling>
```

Where `<chroma-subsampling>` is one of: `420`, `422`, `444`, `j420`, `j422`, `j444`.

## Cross-compiling the CLI

Before cross-compiling the CLI for Windows or static Linux binaries, install the required dependencies using the `install-deps` recipe:

```bash
j install-deps <target> <arch>
```

Where `<target>` is one of: `win`, `linux`, and `<arch>` is one of: `32`, `64`

After that, simply use the `build` recipe to build the CLI for the desired target and architecture.

Simply run:

```bash
j build <target> <arch>
```

Where `<target>` is one of: `win`, `linux`, and `<arch>` is one of: `32`, `64`

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
j build wasm
```

Then build the frontend.

```bash
j build web
```

## Other recipes

Check out the [justfile](../justfile) for other available recipes for development, building, testing, linting, etc.
