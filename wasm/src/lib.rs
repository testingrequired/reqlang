use std::collections::HashMap;

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

#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn resolve(source: &str, env: &str, prompts: JsValue, secrets: JsValue) -> JsValue {
    console_error_panic_hook::set_once();

    let prompts: HashMap<String, String> = serde_wasm_bindgen::from_value(prompts).unwrap();
    let secrets: HashMap<String, String> = serde_wasm_bindgen::from_value(secrets).unwrap();

    let results = parser::resolve(source, env, prompts, secrets);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}
