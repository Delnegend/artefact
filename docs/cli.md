# Artefact CLI Reference

`artefact-cli` is a thin `clap` wrapper around [`artefact-core`](../backend/artefact-core). Every
flag maps directly to a solver parameter; there is no config file.

Defined in [`backend/artefact-cli/main.rs:18`](../backend/artefact-cli/main.rs) and
[`backend/artefact-core/lib.rs:63`](../backend/artefact-core/lib.rs) (`ValueCollection`).

## Flags

| Flag | Short | Default | Description |
|---|---|---|---|
| `<input>` | — | — | Input JPEG file |
| `--output <path>` | `-o` | `<input>.png` | Output file (extension infers format when `--format auto`) |
| `--format <fmt>` | `-f` | `auto` | `auto` or `png`/`webp`/`tiff`/`bmp` |
| `--weight <f32>` | `-w` | `0.3` | 2nd-order weight — higher = smoother, less staircasing. Single or `Y,Cb,Cr` |
| `--pweight <f32>` | `-p` | `0.001` | Fidelity weight — higher = closer to source JPEG |
| `--iterations <n>` | `-i` | `50` | Solver iterations — higher = better but slower. Single or `Y,Cb,Cr` |
| `--separate-components` | `-s` | `false` | Optimize Y/Cb/Cr separately instead of jointly |
| `--gpu` | `-g` | `false` | Use the GPU when available (`process_auto`), otherwise fall back to CPU |
| `--benchmark` | `-b` | `false` | Run solver but don't write output |
| `--overwrite` | `-y` | `false` | Overwrite existing output |

## Examples

```bash
# basic: input.jpg -> input.png (same dir)
artefact-cli input.jpg

# choose output and format
artefact-cli input.jpg -o output.webp --format webp -y

# tune solver (single value for all channels, or Y,Cb,Cr)
artefact-cli input.jpg --weight 0.3 --pweight 0.001 --iterations 50
artefact-cli input.jpg --weight 0.3,0.2,0.3 --iterations 50,30,50

# benchmark without writing file
artefact-cli input.jpg --benchmark

# use the GPU when available, otherwise fall back to the CPU
artefact-cli input.jpg --gpu

# help
artefact-cli --help
```

## Tuning

`--weight` and `--pweight` pull against each other: `--weight` is the 2nd-order smoothness term,
`--pweight` the fidelity term. Raise `--weight` to flatten gradients and kill staircasing; raise
`--pweight` to stay closer to the source JPEG. Defaults (`0.3` / `0.001`) are a good starting
point for photographic content.

Per-channel values are parsed as `Y,Cb,Cr` and applied only when `--separate-components` is set;
otherwise the first value is used for all three. `--iterations` scales quality against runtime
roughly linearly and is the first dial to move when the GPU path is already saturated.

## Backend selection

`process()` always uses the CPU pipeline. The CLI's `--gpu` uses `process_auto()`: it logs the
selected `wgpu` adapter and `solving on GPU` when usable, otherwise it warns and returns to
`solving on CPU pipeline`. Set `RUST_LOG=debug` for lower-level selection diagnostics.

```bash
RUST_LOG=debug artefact-cli input.jpg --gpu --benchmark
```

## Reproducible CPU-vs-GPU comparison

The CLI timing above includes process start and GPU context creation. For an apples-to-apples
solve comparison use the criterion harness instead:

```bash
RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=8 \
  cargo bench -p artefact-core --features bench,simd,gpu --bench gpu
```

See [development.md#benchmarks](development.md#benchmarks) for the matrix selection env vars and
recorded numbers.
