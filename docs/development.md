# Artefact Development Guide

The project contains 2 mains components/directories:

-   [frontend](./frontend/): the web UI built with Nuxt.js
-   [backend](./backend/):
    -   [artefact-lib](./backend/artefact-lib/): the core library that does the image processing
    -   [artefact-cli](./backend/artefact-cli/): the command-line interface that uses artefact-lib
    -   [artefact-wasm](./backend/artefact-wasm/): the WebAssembly bindings for artefact-lib, to be used in the web UI
    -   [zune-jpeg](./backend/zune-jpeg/): a fork of zune-jpeg with some fixes, improvements and DCT coefficients exposed

```bash
cargo build --release --package artefact-cli
```

The binary will be located at ./target/release/artefact-cli

Build features: To toggle specific features when building the CLI, modify artefact-cli/Cargo.toml and add the desired features to the [dependencies.artefact-lib] features list.

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

To build for other platforms, install the required toolchains and libraries using the `install-deps` recipe, e.g.:

```bash
j install-deps win 64
```

Then build using the `build` recipe, e.g.:

```bash
j build win 64
```

### Web UI build guide

Build the WASM library if not already built or if there are changes

```bash
j build wasm
```

Then build the frontend

```bash
j build web
```
