use artefact_lib::{pipeline, Config, DecompressorErr, JpegSource};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The input jpeg file
    #[arg(short, long)]
    input: String,

    /// The output png file
    #[arg(short, long)]
    output: String,

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
    #[arg(short, long)]
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
        Ok(img) => img.save(args.output).expect("Cannot save output image"),
        Err(e) => match e {
            DecompressorErr::DerefNull(ptr) => {
                eprintln!("Trying to dereference null pointer: {}", ptr)
            }

            DecompressorErr::InitJerrErr => eprintln!("Error initializing jpeg error manager"),
            DecompressorErr::InitCinfoErr => eprintln!("Error initializing jpeg decompressor"),

            DecompressorErr::FileNotExist => eprintln!("Input file does not exist"),
            DecompressorErr::FileIsNotFile => eprintln!("Input is not a file"),

            DecompressorErr::SourceNotSet => eprintln!("set_source() not called yet"),
            DecompressorErr::HeaderNotReadYet => eprintln!("read_header() not called yet"),

            DecompressorErr::ParseHeaderErr(e) => eprintln!("Error parsing header: {}", e),
            DecompressorErr::EmptyCoefficientArr => eprintln!("Empty coefficient array"),
            DecompressorErr::UnsupportedNumberOfComponents => {
                eprintln!("Unsupported number of components (must be either 1 or 3)")
            }
            DecompressorErr::AccessVirtualBlockArrayErr => {
                eprintln!("Cannot access virtual block array to extract coefficients")
            }
            DecompressorErr::NoQuantizationTable => eprintln!("Quantization table not found"),

            DecompressorErr::Other(e) => eprintln!("Other error: {}", e),
        },
    }
}
