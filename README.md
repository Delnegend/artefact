<div align="center">

# Artefact

**Reconstructs lost JPEG detail for smoother, cleaner images with reduced compression artifacts.**

[![CI](https://badgers.space/github/checks/Delnegend/artefact/main?corner_radius=0&label=build)](https://github.com/Delnegend/artefact/actions)
[![Release](https://badgers.space/github/release/Delnegend/artefact?corner_radius=0&color=ea7233)](https://github.com/Delnegend/artefact/releases)
[![License](https://badgers.space/badge/license/MIT%20%7C%20Apache-2.0/007ec6?corner_radius=0)](LICENSE-MIT)

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
- **Flexible output**: Exports to PNG, JPEG XL, WebP, TIFF, or BMP. Select 16-bit PNG or JPEG XL to preserve fractional detail.

## Documentation

- **[Architecture](docs/architecture.md)**: Component boundaries, solver pipelines, and repository layout.
- **[Development](docs/development.md)**: Prerequisites, build steps, just check gates, profiling, and [CLI reference](docs/cli.md).
- **[Deployment](docs/deployment.md)**: Installation via Homebrew, prebuilt release archives, and WASM/web deployment.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
