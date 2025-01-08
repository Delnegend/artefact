use std::path::PathBuf;

use artefact_lib::{Artefact, JpegSource, Param};
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

    /// Overwrite existing output file
    #[arg(short = 'y', long, default_value = "false")]
    overwrite: bool,

    /// Second order weight
    /// Higher values give smoother transitions with less staircasing
    ///
    /// Default: 0.3 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment)]
    weight: Option<String>,

    /// Probability weight
    /// Higher values make the result more similar to the source JPEG
    ///
    /// Default: 0.001 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment)]
    pweight: Option<String>,

    /// Iterations
    /// Higher values give better results but take more time
    ///
    /// Default: 50 for all channels, use comma separated values for each channel
    #[arg(short, long, verbatim_doc_comment)]
    iterations: Option<String>,

    /// Separate components
    /// Separately optimize components instead of all together
    ///
    /// Default: false
    #[arg(short, long, verbatim_doc_comment)]
    spearate_components: Option<bool>,
}

fn main() {
    let args = Args::parse();

    let output = args.output.map(PathBuf::from).unwrap_or_else(|| {
        let input_path = PathBuf::from(&args.input);
        input_path.with_extension("png")
    });
    if output.exists() && !args.overwrite {
        eprintln!("Output file already exists, use -y to overwrite");
        return;
    }

    match Artefact::default()
        .source(JpegSource::File(args.input))
        .weight(args.weight.map(|arg| {
            let vals = arg
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid weight value: {}", s))
                })
                .collect::<Vec<f32>>();
            match vals.len() {
                1 => Param::ForAll(vals[0]),
                3 => Param::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of weight values"),
            }
        }))
        .pweight(args.pweight.map(|arg| {
            let vals = arg
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid pweight value: {}", s))
                })
                .collect::<Vec<f32>>();
            match vals.len() {
                1 => Param::ForAll(vals[0]),
                3 => Param::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of pweight values"),
            }
        }))
        .iterations(args.iterations.map(|arg| {
            let vals = arg
                .split(",")
                .map(|s| {
                    s.parse()
                        .unwrap_or_else(|_| panic!("Invalid iterations value: {}", s))
                })
                .collect::<Vec<u32>>();
            match vals.len() {
                1 => Param::ForAll(vals[0]),
                3 => Param::ForEach([vals[0], vals[1], vals[2]]),
                _ => panic!("Invalid number of iterations values"),
            }
        }))
        .process()
    {
        Ok(img) => img.save(output).expect("Cannot save output image"),
        Err(e) => eprintln!("Error: {e:?}"),
    }
}
