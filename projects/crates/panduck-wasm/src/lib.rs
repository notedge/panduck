//! WebAssembly export surface for Panduck.

use wasm_bindgen::prelude::*;

const FORMATS: [&str; 4] = ["markdown", "org", "rst", "tex"];

/// Returns the Panduck WASM binding version.
#[wasm_bindgen(js_name = panduckVersion)]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists format adapters wired into this workspace.
#[wasm_bindgen(js_name = supportedFormats)]
pub fn supported_formats() -> Vec<JsValue> {
    FORMATS
        .iter()
        .map(|name| JsValue::from_str(name))
        .collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[wasm_bindgen(js_name = isSupportedFormat)]
pub fn is_supported_format(name: &str) -> bool {
    FORMATS
        .iter()
        .any(|format| format.eq_ignore_ascii_case(name))
}
