use std::path::PathBuf;
use std::process::ExitCode;

use artefact_core::{
    Artefact, ArtefactError, JpegSource, OutputFormat, ValueCollection, encode_image,
};
use clap::Parser;

/// Version reported by `--version`.
///
/// The release workflow (and the Homebrew formula) inject
/// `ARTEFACT_BUILD_VERSION`; local builds fall back to the crate version.
const VERSION: &str = match option_env!("ARTEFACT_BUILD_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

#[derive(Parser, Debug)]
#[command(version = VERSION, about)]
struct Args {
    /// The input jpeg file
    #[arg(index = 1)]
    input: String,

    /// The output file
    #[arg(short, long)]
    output: Option<String>,

    /// Output format (auto, png, webp, tiff, bmp, jxl)
    #[arg(short, long, default_value = "auto")]
    format: String,

    /// Preserve fractional detail in a 16-bit PNG or JPEG XL
    #[arg(long)]
    sixteen_bit: bool,

    /// Overwrite existing output file
    #[arg(short = 'y', long, default_value = "false")]
    overwrite: bool,

    /// Higher second order weight give smoother transitions with less staircasing
    ///
    /// Use comma separated values for each channel
    #[arg(short, long, default_value = "0.3")]
    weight: String,

    /// Higher probability weight make the result more similar to the source JPEG
    ///
    /// Use comma separated values for each channel
    #[arg(short, long, default_value = "0.001")]
    pweight: String,

    /// Higher iteration give better results but take more time
    ///
    /// Use comma separated values for each channel
    #[arg(short, long, default_value = "50")]
    iterations: String,

    #[command(flatten)]
    flags: RunFlags,
}

#[derive(clap::Args, Debug)]
struct RunFlags {
    /// Separately optimize components instead of all together
    #[arg(short, long, default_value = "false", alias = "spearate-components")]
    separate_components: bool,

    /// Benchmark mode, do not save output image
    #[arg(short, long, default_value = "false")]
    benchmark: bool,

    /// Use the GPU pipeline, falling back to CPU when no adapter is available
    #[arg(short, long, default_value = "false")]
    gpu: bool,
}

const POSSIBLE_FORMATS: [&str; 5] = ["png", "webp", "tiff", "bmp", "jxl"];

/// Parse `1` or `3` comma-separated values into a [`ValueCollection`].
fn parse_values<T>(raw: &str, label: &str) -> Result<ValueCollection<T>, String>
where
    T: std::str::FromStr + Copy,
{
    let vals = raw
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<T>()
                .map_err(|_| format!("invalid {label} value: {s}"))
        })
        .collect::<Result<Vec<T>, String>>()?;

    match vals.as_slice() {
        [one] => Ok(ValueCollection::ForAll(*one)),
        [a, b, c] => Ok(ValueCollection::ForEach([*a, *b, *c])),
        _ => Err(format!(
            "{label} expects 1 or 3 comma-separated values, got {}",
            vals.len()
        )),
    }
}

/// Resolve the output path and format, validating explicit formats/extensions.
fn resolve_output(args: &Args) -> Result<(PathBuf, String), String> {
    let ext = args
        .output
        .as_deref()
        .map(PathBuf::from)
        .and_then(|p| p.extension().map(|e| e.to_string_lossy().to_lowercase()));

    let format = if args.format == "auto" {
        match &ext {
            Some(e) if POSSIBLE_FORMATS.contains(&e.as_str()) => e.clone(),
            Some(e) => {
                return Err(format!(
                    "cannot infer format from extension .{e}; use --format"
                ));
            }
            None => "png".to_string(),
        }
    } else {
        if !POSSIBLE_FORMATS.contains(&args.format.as_str()) {
            return Err(format!(
                "invalid output format ({}), possible values: {}",
                args.format,
                POSSIBLE_FORMATS.join(", ")
            ));
        }
        if let Some(e) = &ext
            && *e != args.format.to_lowercase()
        {
            return Err(format!(
                "output extension (.{e}) does not match --format {}",
                args.format
            ));
        }
        args.format.clone()
    };

    let path = args.output.as_ref().map_or_else(
        || PathBuf::from(&args.input).with_extension(&format),
        |out| {
            let p = PathBuf::from(out);
            if p.extension().is_some() {
                p
            } else {
                p.with_extension(&format)
            }
        },
    );

    Ok((path, format))
}

fn main() -> ExitCode {
    init_tracing();
    match run(&Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Initialise `tracing` (honours `RUST_LOG`, defaults to `info`).
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();
}

fn run(args: &Args) -> Result<(), String> {
    let (output, format) = resolve_output(args)?;
    if args.sixteen_bit && format != "png" && format != "jxl" {
        return Err("--sixteen-bit requires PNG or JPEG XL output".into());
    }
    let output_format = match format.as_str() {
        "png" => OutputFormat::Png,
        "webp" => OutputFormat::Webp,
        "tiff" => OutputFormat::Tiff,
        "bmp" => OutputFormat::Bmp,
        "jxl" => OutputFormat::Jxl,
        _ => return Err(format!("invalid output format: {format}")),
    };

    if output.exists() && !args.overwrite && !args.flags.benchmark {
        return Err("output file already exists, use -y to overwrite".into());
    }

    let artefact = Artefact::default()
        .source(JpegSource::File(args.input.clone()))
        .weight(parse_values::<f32>(&args.weight, "weight")?)
        .pweight(parse_values::<f32>(&args.pweight, "pweight")?)
        .iterations(parse_values::<usize>(&args.iterations, "iterations")?)
        .benchmark(args.flags.benchmark)
        .separate_components(args.flags.separate_components);

    if args.sixteen_bit {
        let result = if args.flags.gpu {
            pollster::block_on(artefact.process_auto_16())
        } else {
            artefact.process_16()
        };
        return match result {
            Ok(img) => save_image(&output, &img.into(), output_format),
            Err(ArtefactError::Benchmark) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }

    let result = if args.flags.gpu {
        pollster::block_on(artefact.process_auto())
    } else {
        artefact.process()
    };

    match result {
        Ok(img) => save_image(&output, &img.into(), output_format),
        Err(ArtefactError::Benchmark) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn save_image(
    path: &std::path::Path,
    image: &artefact_core::image::DynamicImage,
    format: OutputFormat,
) -> Result<(), String> {
    let file =
        std::fs::File::create(path).map_err(|e| format!("cannot save {}: {e}", path.display()))?;
    let mut output = std::io::BufWriter::new(file);
    encode_image(image, format, &mut output)
        .map_err(|e| format!("cannot save {}: {e}", path.display()))?;
    std::io::Write::flush(&mut output).map_err(|e| format!("cannot save {}: {e}", path.display()))
}
