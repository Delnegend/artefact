# artefact

## What this does

JPEG compression discards image data. Regular decoders then try to "fill in" the missing pieces, and that guessing often creates visible artifacts.

This project, instead of patching missing information with noisy guesses, reconstructs the lost detail in a way that produces smoother, more visually pleasing images.

This project is based on [jpeg2png](https://github.com/victorvde/jpeg2png), and:

-   <img src="./assets/rust.svg" width=18 align="center"> is rewritten in Rust.
-   ⚡ is nearly 3× faster.
-   <img src="./assets/wasm.svg" width=18 align="center"> is WASM-ready, runs directly in the browser.

## Demos

![](assets/01.png)
![](assets/02.png)

> [Photo by Aleksandar Pasaric](https://www.pexels.com/photo/photo-of-neon-signage-1820770/)

![](assets/03.png)
![](assets/04.png)

> [Photo by Toa Heftiba Şinca](https://www.pexels.com/photo/selective-photograph-of-a-wall-with-grafitti-1194420/)

## Quick start

There are two ways to use artefact:

### 1. The performance way

Either download the latest binary from the [releases page](https://github.com/Delnegend/artefact/releases/latest) or [build from source](#cli-build-guide).

To see all available CLI options run:

```
artefact-cli --help
```

TL;DR:

```
artefact-cli input.jpg
```

### 2. The convenience way

Go to [artefact.delnegend.com](https://artefact.delnegend.com/), upload your JPEG image, and hit the "Process" button.

> Everything runs directly in your browser, no data is sent anywhere. The downside is that processing is slower than the native binary.

## Resources

-   [Development guide](./docs/development.md)

## License

Licensed under either of

-   Apache License, Version 2.0 ([LICENSE-Apache](./LICENSE-Apache) or [apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
-   MIT license ([LICENSE-MIT](./LICENSE-MIT) or [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))
    at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
