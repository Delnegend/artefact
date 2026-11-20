use std::io::Cursor;

use artefact_core::{Artefact, JpegSource, ValueCollection, image::ImageFormat};
use wasm_bindgen::prelude::*;

#[allow(clippy::float_cmp)]
mod output_format {
    use wasm_bindgen::prelude::*;

    // `#[wasm_bindgen]` on C-style enums emits an exact float comparison in 0.2.130+.
    #[derive(Clone, Copy)]
    #[wasm_bindgen]
    pub enum OutputFormat {
        Png,
        Webp,
        Tiff,
        Bmp,
    }
}

pub use output_format::OutputFormat;

#[wasm_bindgen]
/// Run JPEG artefact removal on `buffer` and encode the result in `output_format`.
///
pub fn compute(
    buffer: Vec<u8>,
    output_format: OutputFormat,
    weight: f32,
    pweight: f32,
    iterations: usize,
    separate_components: bool,
    use_gpu: bool,
) -> wasm_bindgen_futures::js_sys::Promise {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let output_format = match output_format {
        OutputFormat::Png => ImageFormat::Png,
        OutputFormat::Webp => ImageFormat::WebP,
        OutputFormat::Tiff => ImageFormat::Tiff,
        OutputFormat::Bmp => ImageFormat::Bmp,
    };

    let mut cursor = Cursor::new(Vec::new());

    let artefact = Artefact::default()
        .source(JpegSource::Buffer(buffer))
        .weight(ValueCollection::ForAll(weight))
        .pweight(ValueCollection::ForAll(pweight))
        .iterations(ValueCollection::ForAll(iterations))
        .separate_components(separate_components);

    wasm_bindgen_futures::future_to_promise(async move {
        // `use_gpu` is decided by the caller: the browser can expose `navigator.gpu`
        // while `requestAdapter()` resolves to `null`, which wgpu 30 mishandles.
        let image = if use_gpu {
            artefact.process_auto().await
        } else {
            artefact.process()
        }
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        image
            .write_to(&mut cursor, output_format)
            .map_err(|e| JsValue::from_str(&format!("Can't write image to buffer: {e:?}")))?;

        let bytes = cursor.into_inner();
        Ok(wasm_bindgen_futures::js_sys::Uint8Array::from(bytes.as_slice()).into())
    })
}
