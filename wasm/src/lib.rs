use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parse(source: &str) -> JsValue {
    console_error_panic_hook::set_once();

    let results = parser::parse(source);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}
