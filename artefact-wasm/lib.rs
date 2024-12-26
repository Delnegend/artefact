use std::io::Cursor;

use artefact_lib::{pipeline, JpegSource};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute(
    // weight: Vec<f32>,
    // pweight: Vec<f32>,
    // iterations: Vec<u32>,
    // separate_components: bool,
    buffer: Vec<u8>,
) -> Result<Vec<u8>, String> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut cursor = Cursor::new(Vec::new());

    // Some(Config {
    //     weight: weight.try_into().unwrap(),
    //     pweight: pweight.try_into().unwrap(),
    //     iterations: iterations.try_into().unwrap(),
    //     separate_components,
    // }),

    pipeline(None, JpegSource::Buffer(buffer))?
        .write_to(&mut cursor, artefact_lib::image::ImageFormat::Png)
        .map_err(|e| format!("Can't write image to buffer: {e:?}",))?;

    Ok(cursor.into_inner())
}
