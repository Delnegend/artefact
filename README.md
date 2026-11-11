<div align="center">

# Artefact

**Reconstructs lost JPEG detail for smoother, cleaner images with reduced compression artifacts.**

[![CI](https://img.shields.io/github/actions/workflow/status/Delnegend/artefact/ci.yml?branch=main&style=flat-square)](https://github.com/Delnegend/artefact/actions)
[![Release](https://img.shields.io/github/v/release/Delnegend/artefact?style=flat-square)](https://github.com/Delnegend/artefact/releases)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue?style=flat-square)](LICENSE-MIT)

</div>

---

## Quick Start

```bash
# 1. Install
brew install Delnegend/tap/artefact-cli

# 2. Reconstruct an image
artefact-cli input.jpg -o output.png

# 3. Web browser (zero install)
# Visit https://artefact.delnegend.com to drop and compare images locally with WebAssembly/WebGPU.
```

## Highlights

- **Artifact reduction**: Re-optimizes raw DCT coefficients with a regularized solver rather than making pixel-level guesses.
- **High throughput**: Multi-threaded execution with adaptive SIMD vectorization and optional WebGPU acceleration.
- **Cross-platform**: Runs as a single static native CLI or entirely client-side in the browser via WebAssembly (zero uploads).
- **Flexible output**: Ingests standard `.jpg`/`.jpeg` files and exports to PNG, WebP, TIFF, or BMP.

## Documentation

- **[Architecture](docs/architecture.md)**: Component boundaries, solver pipelines, and repository layout.
- **[Development](docs/development.md)**: Prerequisites, build steps, just check gates, and profiling.
- **[Deployment](docs/deployment.md)**: Installation via Homebrew, prebuilt release archives, and WASM/web deployment.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
