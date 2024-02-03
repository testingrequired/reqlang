use std::collections::HashMap;

use export::Format;
use wasm_bindgen::prelude::*;

/// Parse a string in to a request file with unresoling template values
///
/// # Arguments
///
/// * `source` - String to parse
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

/// Get the env names from a request file
///
/// # Arguments
///
/// * `source` - String to parse
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn getEnvNames(source: &str) -> JsValue {
    console_error_panic_hook::set_once();

    let results = parser::parse(source);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results.env_names()).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}

/// Parse a string in to a request file and resolve template values
///
/// # Arguments
///
/// * `source` - String to parse
/// * `env` - Environment to use when resolving variables
/// * `prompts` - Prompt values
/// * `secrets` - Secret values
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn resolve(source: &str, env: &str, prompts: JsValue, secrets: JsValue) -> JsValue {
    console_error_panic_hook::set_once();

    let prompts: HashMap<String, String> = serde_wasm_bindgen::from_value(prompts).unwrap();
    let secrets: HashMap<String, String> = serde_wasm_bindgen::from_value(secrets).unwrap();

    let results = parser::resolve(source, env, &prompts, &secrets);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}

/// Parse a string in to a request file and resolve template values
///
/// # Arguments
///
/// * `source` - String to parse
/// * `env` - Environment to use when resolving variables
/// * `prompts` - Prompt values
/// * `secrets` - Secret values
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn template(source: &str, env: &str, prompts: JsValue, secrets: JsValue) -> JsValue {
    console_error_panic_hook::set_once();

    let prompts: HashMap<String, String> = serde_wasm_bindgen::from_value(prompts).unwrap();
    let secrets: HashMap<String, String> = serde_wasm_bindgen::from_value(secrets).unwrap();

    let results = parser::template(source, env, &prompts, &secrets);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}

/// Export a request file in another format
///
/// # Arguments
///
/// * `source` - String to parse
/// * `env` - Environment to use when resolving variables
/// * `prompts` - Prompt values
/// * `secrets` - Secret values
/// * `format` - The format to export the request file in: `Http`, `Curl`
#[wasm_bindgen]
#[allow(non_snake_case)]
pub fn export_to_format(
    source: &str,
    env: &str,
    prompts: JsValue,
    secrets: JsValue,
    format: JsValue,
) -> JsValue {
    console_error_panic_hook::set_once();

    let prompts: HashMap<String, String> = serde_wasm_bindgen::from_value(prompts).unwrap();
    let secrets: HashMap<String, String> = serde_wasm_bindgen::from_value(secrets).unwrap();
    let format: Format = serde_wasm_bindgen::from_value(format).unwrap();

    let results = parser::export(source, env, &prompts, &secrets, format);

    match results {
        Ok(results) => serde_wasm_bindgen::to_value(&results).unwrap(),
        Err(err) => serde_wasm_bindgen::to_value(&err).unwrap(),
    }
}
