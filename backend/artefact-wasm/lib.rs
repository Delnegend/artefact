use std::io::Cursor;

use artefact_core::{Artefact, JpegSource, OutputFormat as CoreFormat, ValueCollection, encode_image};
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
        Jxl,
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
    output_16_bit: bool,
) -> wasm_bindgen_futures::js_sys::Promise {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let format = match output_format {
        OutputFormat::Png => CoreFormat::Png,
        OutputFormat::Webp => CoreFormat::Webp,
        OutputFormat::Tiff => CoreFormat::Tiff,
        OutputFormat::Bmp => CoreFormat::Bmp,
        OutputFormat::Jxl => CoreFormat::Jxl,
    };

    let artefact = Artefact::default()
        .source(JpegSource::Buffer(buffer))
        .weight(ValueCollection::ForAll(weight))
        .pweight(ValueCollection::ForAll(pweight))
        .iterations(ValueCollection::ForAll(iterations))
        .separate_components(separate_components);

    wasm_bindgen_futures::future_to_promise(async move {
        if output_16_bit && !matches!(output_format, OutputFormat::Png | OutputFormat::Jxl) {
            return Err(JsValue::from_str("16-bit output is only supported for PNG and JPEG XL"));
        }
        // `use_gpu` is decided by the caller: the browser can expose `navigator.gpu`
        // while `requestAdapter()` resolves to `null`, which wgpu 30 mishandles.
        let image = if output_16_bit {
            let image = if use_gpu {
                artefact.process_auto_16().await
            } else {
                artefact.process_16()
            }.map_err(|e| JsValue::from_str(&e.to_string()))?;
            image.into()
        } else {
            let image = if use_gpu {
                artefact.process_auto().await
            } else {
                artefact.process()
            }.map_err(|e| JsValue::from_str(&e.to_string()))?;
            image.into()
        };
        let mut output = Cursor::new(Vec::new());
        encode_image(&image, format, &mut output).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let result = wasm_bindgen_futures::js_sys::Array::new();
        result.push(&wasm_bindgen_futures::js_sys::Uint8Array::from(output.into_inner().as_slice()));
        if format == CoreFormat::Jxl {
            let mut preview = Cursor::new(Vec::new());
            encode_image(&image, CoreFormat::Png, &mut preview).map_err(|e| JsValue::from_str(&e.to_string()))?;
            result.push(&wasm_bindgen_futures::js_sys::Uint8Array::from(preview.into_inner().as_slice()));
        } else {
            result.push(&JsValue::NULL);
        }
        Ok(result.into())
    })
}
