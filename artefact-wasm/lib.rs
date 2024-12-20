use artefact_lib::{pipeline, Config, JpegSource};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn compute(
    weight: Vec<f32>,
    pweight: Vec<f32>,
    iterations: Vec<u32>,
    separate_components: bool,
    buffer: Vec<u8>,
) -> Result<Vec<u8>, JsValue> {
    pipeline(
        Some(Config {
            weight: weight.try_into().unwrap(),
            pweight: pweight.try_into().unwrap(),
            iterations: iterations.try_into().unwrap(),
            separate_components,
        }),
        JpegSource::Buffer(buffer),
    )
    .map_err(|e| JsValue::from_str(&format!("{:?}", e)))
    .map(|img| img.into_raw())
}
