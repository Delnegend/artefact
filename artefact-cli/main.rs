use std::path::PathBuf;

use artefact_lib::{Artefact, JpegSource, ValueCollection};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The input jpeg file
    #[arg(index = 1)]
    input: String,

    /// The output png file
    ///
    /// Default: input file with png extension
    #[arg(short, long)]
    output: Option<String>,

    /// Output format
    ///
    /// Default: png
    /// Possible values: png, webp, tiff, bmp, gif
    #[arg(short, long, default_value = "png")]
    format: String,

    /// Overwrite existing output file
    #[arg(short = 'y', long, default_value = "false")]
    overwrite: bool,

    /// Second order weight
    /// Higher values give smoother transitions with less staircasing
    ///
    /// Default: 0.3 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment, default_value = "0.3")]
    weight: String,

    /// Probability weight
    /// Higher values make the result more similar to the source JPEG
    ///
    /// Default: 0.001 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment, default_value = "0.001")]
    pweight: String,

    /// Iterations
    /// Higher values give better results but take more time
    ///
    /// Default: 100 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment, default_value = "100")]
    iterations: String,

    /// Separate components
    /// Separately optimize components instead of all together, exchanges quality for speed
    #[arg(short, long, verbatim_doc_comment, default_value = "false")]
    spearate_components: bool,

    /// Benchmark mode, do not save output image
    #[arg(short, long, verbatim_doc_comment, default_value = "false")]
    benchmark: bool,
}

fn main() {
    let args = Args::parse();

    if !["png", "webp", "tiff", "bmp", "gif"].contains(&args.format.as_str()) {
        eprintln!("Invalid output format. Possible values: png, webp, tiff, bmp, gif");
        return;
    }

    let output = args.output.map(PathBuf::from).unwrap_or_else(|| {
        let input_path = PathBuf::from(&args.input);
        input_path.with_extension(args.format)
    });
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
