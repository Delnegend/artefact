use std::io::Cursor;

use artefact_lib::{image::ImageFormat, Artefact, JpegSource, ValueCollection};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub enum OutputFormat {
    Png,
    Webp,
    Tiff,
    Bmp,
}

#[wasm_bindgen]
pub fn compute(
    buffer: Vec<u8>,
    output_format: Option<OutputFormat>,
    weight: Option<f32>,
    pweight: Option<f32>,
    iterations: Option<u32>,
    separate_components: Option<bool>,
) -> Result<Vec<u8>, String> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let output_format = match output_format {
        Some(OutputFormat::Png) => ImageFormat::Png,
        Some(OutputFormat::Webp) => ImageFormat::WebP,
        Some(OutputFormat::Tiff) => ImageFormat::Tiff,
        Some(OutputFormat::Bmp) => ImageFormat::Bmp,
        None => ImageFormat::Png,
    };

    let mut cursor = Cursor::new(Vec::new());

    Artefact::default()
        .source(JpegSource::Buffer(buffer))
        .weight(weight.map(ValueCollection::ForAll))
        .pweight(pweight.map(ValueCollection::ForAll))
        .iterations(iterations.map(ValueCollection::ForAll))
        .separate_components(separate_components)
        .process()?
        .write_to(&mut cursor, output_format)
        .map_err(|e| format!("Can't write image to buffer: {e:?}",))?;

    Ok(cursor.into_inner())
}
