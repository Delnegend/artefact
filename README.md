# artefact

No AI, no machine learning, just un-rounding numbers to achieve the smoothest, compression-artifact-free images from JPEGs.

> Works best on illustrations but cannot help with JPEGs that have been re-encoded multiple times.
>
> This application is a pure Rust 🦀 port of the original [jpeg2png](https://github.com/ThioJoe/jpeg2png/) project.

## Results

|![](assets/01.png)|
|:---:|
|Image by [ANDRI TEGAR MAHARDIKA](https://pixabay.com/users/andsproject-26081561/) from [Pixabay](https://pixabay.com/)|

|![](assets/02.png)|
|:---:|
|Image by [HANSUAN FABREGAS](https://pixabay.com/users/hansuan_fabregas-2902307/) from [Pixabay](https://pixabay.com/)|

|![](assets/03.png)|
|:---:|
|Image [fruits.bmp](https://www.hlevkin.com/hlevkin/TestImages/fruits.bmp) from <a href="https://www.hlevkin.com/hlevkin/06testimages.htm">Mathship Technologies</a>|

## Web version
WIP

## CLI version

### Build
```bash
cargo build --release --package artefact-cli
```

### Usage
```
Usage: artefact-cli.exe [OPTIONS] --input <INPUT>

Options:
  -i, --input <INPUT>                              The input jpeg file
  -o, --output <OUTPUT>                            The output png file
  -y, --overwrite                                  Overwrite existing output file
  -w, --weight <WEIGHT>                            Second order weight
  -p, --pweight <PWEIGHT>                          Probability weight
      --iterations <ITERATIONS>                    Iterations
  -s, --spearate-components <SPEARATE_COMPONENTS>  Separate components [possible values: true, false]
  -h, --help                                       Print help (see more with '--help')
  -V, --version                                    Print version
```

## Developement
- [vscode](https://code.visualstudio.com/) + [`devcontainer`](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)

### Directories
- [`artefact-lib`](./artefact-lib/) - the implementation and pipeline
- [`artefact-cli`](./artefact-cli/) - command-line interface wrapper
- [`artefact-wasm`](./artefact-wasm/) - the [`wasm-pack`](https://github.com/rustwasm/wasm-pack) wrapper designed to build WebAssembly (WASM) modules compatible with modern browsers (work in progress)
- [`zune-jpeg`](./zune-jpeg/) - a modified fork of [`zune-jpeg`](https://github.com/etemesi254/zune-image/tree/dev/crates/zune-jpeg) exposes the underlying DCT coefficients and quantization tables, replacing the original `libjpeg` implementation.

The binary will be located at `./target/release/artefact-cli`

## License
Licensed under either of

- Apache License, Version 2.0 ([LICENSE-Apache](./LICENSE-Apache) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](./LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.