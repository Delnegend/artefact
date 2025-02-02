use std::path::PathBuf;

use artefact_lib::{Artefact, JpegSource, ValueCollection};
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

    /// Output format (auto, png, webp, tiff, bmp, gif)
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
    #[arg(short, long, default_value = "false")]
    spearate_components: bool,

    /// Benchmark mode, do not save output image
    #[arg(short, long, default_value = "false")]
    benchmark: bool,
}

const POSSIBLE_FORMATS: [&str; 5] = ["png", "webp", "tiff", "bmp", "gif"];

fn main() {
    let args = Args::parse();

    let output = {
        let final_format = match (&args.format, &args.output) {
            (f, Some(output)) if f == "auto" => {
                let output = PathBuf::from(output);
                output
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string())
                    .unwrap_or_else(|| "png".to_string())
            }
            (f, None) if f == "auto" => "png".to_string(),
            (f, _) => {
                if !POSSIBLE_FORMATS.contains(&f.as_str()) {
                    eprintln!(
                        "Invalid output format ({f}), possible values: {}",
                        POSSIBLE_FORMATS.join(", ")
                    );
                    return;
                }
                f.clone()
            }
        };

        match args.output.map(PathBuf::from).map(|p| {
            (
                p.extension().map(|ext| ext.to_string_lossy().to_string()),
                p,
            )
        }) {
            Some((Some(output_ext), output_path)) => {
                if args.format != "auto" && output_ext != final_format {
                    eprintln!("Output file extension does not match output format");
                    return;
                }
                output_path
            }
            Some((None, output)) => output.with_extension(&final_format),
            _ => {
                let input_path = PathBuf::from(&args.input);
                input_path.with_extension(&final_format)
            }
        }
    };

    if output.exists() && !args.overwrite && !args.benchmark {
        eprintln!("Output file already exists, use -y to overwrite");
        return;
    }

    match Artefact::default()
        .source(JpegSource::File(args.input))
        .weight({
            let vals = args
                .weight
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid weight value: {}", s))
                })
                .collect::<Vec<f32>>();
            match vals.len() {
                1 => ValueCollection::ForAll(vals[0]),
                3 => ValueCollection::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of weight values"),
            }
        })
        .pweight({
            let vals = args
                .pweight
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid pweight value: {}", s))
                })
                .collect::<Vec<f32>>();
            match vals.len() {
                1 => ValueCollection::ForAll(vals[0]),
                3 => ValueCollection::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of pweight values"),
            }
        })
        .iterations({
            let vals = args
                .iterations
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid iterations value: {}", s))
                })
                .collect::<Vec<usize>>();
            match vals.len() {
                1 => ValueCollection::ForAll(vals[0]),
                3 => ValueCollection::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of iterations values"),
            }
        })
        .benchmark(args.benchmark)
        .separate_components(args.spearate_components)
        .process()
    {
        Ok(img) => img.save(output).expect("Cannot save output image"),
        Err(e) => {
            if e == "BENCHMARK" {
                return;
            }
            eprintln!("Error: {e:?}");
        }
    }
}
