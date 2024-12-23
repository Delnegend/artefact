# artefact

Intercept the JPEG image decoding process to achieve an artifact-free output.

Pure Rust port of the [jpeg2png](https://github.com/ThioJoe/jpeg2png/)

## Directories
- [`artefact-lib`](./artefact-lib/) - the implementation and pipeline
- [`artefact-cli`](./artefact-cli/) - command-line interface wrapper
- [`artefact-wasm`](./artefact-wasm/) - the [`wasm-pack`](https://github.com/rustwasm/wasm-pack) wrapper designed to build WebAssembly (WASM) modules compatible with modern browsers (work in progress)
- [`zune-jpeg`](./zune-jpeg/) - a modified fork of [`zune-jpeg`](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) exposes the underlying DCT coefficients and quantization tables, replacing the currently using [`mozjpeg-sys`](https://github.com/kornelski/mozjpeg-sys) crate (work in progress).

## Developement
- [vscode](https://code.visualstudio.com/) + [`devcontainer`](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

## Build
```bash
cargo build --release --package artefact-cli
```

The binary will be located at `./target/release/artefact-cli`
