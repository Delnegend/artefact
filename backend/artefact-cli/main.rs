use std::path::PathBuf;
use std::process::ExitCode;

use artefact_core::{Artefact, ArtefactError, JpegSource, ValueCollection};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The input jpeg file
    #[arg(index = 1)]
    input: String,

    /// The output file
    #[arg(short, long)]
    output: Option<String>,

    /// Output format (auto, png, webp, tiff, bmp)
    #[arg(short, long, default_value = "auto")]
    format: String,

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

const POSSIBLE_FORMATS: [&str; 4] = ["png", "webp", "tiff", "bmp"];

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

    let path = match &args.output {
        Some(out) => {
            let p = PathBuf::from(out);
            if p.extension().is_some() {
                p
            } else {
                p.with_extension(&format)
            }
        }
        None => PathBuf::from(&args.input).with_extension(&format),
    };

    Ok((path, format))
}

fn main() -> ExitCode {
    init_tracing();
    match run(Args::parse()) {
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

fn run(args: Args) -> Result<(), String> {
    let (output, _format) = resolve_output(&args)?;

    if output.exists() && !args.overwrite && !args.benchmark {
        return Err("output file already exists, use -y to overwrite".into());
    }

    let artefact = Artefact::default()
        .source(JpegSource::File(args.input.clone()))
        .weight(parse_values::<f32>(&args.weight, "weight")?)
        .pweight(parse_values::<f32>(&args.pweight, "pweight")?)
        .iterations(parse_values::<usize>(&args.iterations, "iterations")?)
        .benchmark(args.benchmark)
        .separate_components(args.separate_components);

    let result = if args.gpu {
        pollster::block_on(artefact.process_auto())
    } else {
        artefact.process()
    };

    match result {
        Ok(img) => img
            .save(&output)
            .map_err(|e| format!("cannot save {}: {e}", output.display())),
        Err(ArtefactError::Benchmark) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
