use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parse(source: &str) -> JsValue {
    console_error_panic_hook::set_once();

    let results = parser::parse(source);
    let r = results.unwrap();

    serde_wasm_bindgen::to_value(&r).unwrap()
}
