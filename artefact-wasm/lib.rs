use std::io::Cursor;

use artefact_lib::{pipeline, Config, JpegSource};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute(
    weight: Option<f32>,
    pweight: Option<f32>,
    iterations: Option<u32>,
    separate_components: Option<bool>,
    buffer: Vec<u8>,
) -> Result<Vec<u8>, String> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let mut cursor = Cursor::new(Vec::new());
    let mut config = Config::default();

    if let Some(weight) = weight {
        config.weight = [weight, weight, weight];
    }

    if let Some(pweight) = pweight {
        config.pweight = [pweight, pweight, pweight];
    }

    if let Some(iterations) = iterations {
        config.iterations = [iterations, iterations, iterations];
    }

    if let Some(separate_components) = separate_components {
        config.separate_components = separate_components;
    }

    pipeline(Some(config), JpegSource::Buffer(buffer))?
        .write_to(&mut cursor, artefact_lib::image::ImageFormat::Png)
        .map_err(|e| format!("Can't write image to buffer: {e:?}",))?;

    Ok(cursor.into_inner())
}
