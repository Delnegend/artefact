use std::io::Cursor;

use artefact_lib::{Artefact, JpegSource, Param};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute(
    buffer: Vec<u8>,
    weight: Option<f32>,
    pweight: Option<f32>,
    iterations: Option<u32>,
    separate_components: Option<bool>,
) -> Result<Vec<u8>, String> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut cursor = Cursor::new(Vec::new());

    Artefact::default()
        .source(JpegSource::Buffer(buffer))
        .weight(weight.map(Param::ForAll))
        .pweight(pweight.map(Param::ForAll))
        .iterations(iterations.map(Param::ForAll))
        .separate_components(separate_components)
        .process()?
        .write_to(&mut cursor, artefact_lib::image::ImageFormat::Png)
        .map_err(|e| format!("Can't write image to buffer: {e:?}",))?;

    Ok(cursor.into_inner())
}
