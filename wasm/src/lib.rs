use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn parse(source: &str) -> String {
    console_error_panic_hook::set_once();

    let results = parser::parse(source);

    format!("{:#?}", results)
}
