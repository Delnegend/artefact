use std::io::Cursor;

use artefact_lib::{Artefact, JpegSource, ValueCollection, image::ImageFormat};
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
    output_format: OutputFormat,
    weight: f32,
    pweight: f32,
    iterations: usize,
    separate_components: bool,
) -> Result<Vec<u8>, String> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let output_format = match output_format {
        OutputFormat::Png => ImageFormat::Png,
        OutputFormat::Webp => ImageFormat::WebP,
        OutputFormat::Tiff => ImageFormat::Tiff,
        OutputFormat::Bmp => ImageFormat::Bmp,
    };

    let mut cursor = Cursor::new(Vec::new());

    Artefact::default()
        .source(JpegSource::Buffer(buffer))
        .weight(ValueCollection::ForAll(weight))
        .pweight(ValueCollection::ForAll(pweight))
        .iterations(ValueCollection::ForAll(iterations))
        .separate_components(separate_components)
        .process()?
        .write_to(&mut cursor, output_format)
        .map_err(|e| format!("Can't write image to buffer: {e:?}",))?;

    Ok(cursor.into_inner())
}
