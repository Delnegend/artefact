use std::path::PathBuf;

use artefact_lib::{pipeline, Config, JpegSource};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The input jpeg file
    #[arg(short, long)]
    input: String,

    /// The output png file
    #[arg(short, long)]
    output: Option<String>,

    /// Overwrite existing output file
    #[arg(short = 'y', long, default_value = "false")]
    overwrite: bool,

    /// Second order weight
    ///
    /// Higher values give smoother transitions with less staircasing
    #[arg(short, long)]
    weight: Option<String>,

    /// Probability weight
    ///
    /// Higher values make the result more similar to the source JPEG
    ///
    /// Default: 0.001 for all channels, use comma separated values for each channel
    #[arg(short, long)]
    pweight: Option<String>,

    /// Iterations
    ///
    /// Higher values give better results but take more time
    ///
    /// Default: 50 for all channels, use comma separated values for each channel
    #[arg(long)]
    iterations: Option<String>,

    /// Separate components
    ///
    /// Separately optimize components instead of all together
    ///
    /// Default: false
    #[arg(short, long)]
    spearate_components: Option<bool>,
}

fn main() {
    let args = Args::parse();

    let weight: Option<[f32; 3]> = args.weight.map(|w| {
        w.split(",")
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|_| panic!("Invalid weight value: {}", s))
            })
            .take(3)
            .collect::<Vec<f32>>()
            .try_into()
            .expect("Invalid number of weight values")
    });

    let pweight: Option<[f32; 3]> = args.pweight.map(|w| {
        w.split(",")
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|_| panic!("Invalid pweight value: {}", s))
            })
            .take(3)
            .collect::<Vec<f32>>()
            .try_into()
            .expect("Invalid number of pweight values")
    });

    let iterations: Option<[u32; 3]> = args.iterations.map(|w| {
        w.split(",")
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|_| panic!("Invalid iterations value: {}", s))
            })
            .take(3)
            .collect::<Vec<u32>>()
            .try_into()
            .expect("Invalid number of iterations values")
    });

    let output = args.output.map(PathBuf::from).unwrap_or_else(|| {
        let input_path = PathBuf::from(&args.input);
        input_path.with_extension("png")
    });
    if output.exists() && !args.overwrite {
        eprintln!("Output file already exists, use -y to overwrite");
        return;
    }

    let mut config = Config::default();
    if let Some(weight) = weight {
        config.weight = weight;
    }
    if let Some(pweight) = pweight {
        config.pweight = pweight;
    }
    if let Some(iterations) = iterations {
        config.iterations = iterations;
    }
    if let Some(spearate_components) = args.spearate_components {
        config.separate_components = spearate_components;
    }

    match pipeline(Some(Config::default()), JpegSource::File(args.input)) {
        Ok(img) => img.save(output).expect("Cannot save output image"),
        Err(e) => eprintln!("Error: {e:?}"),
    }
}
