#![deny(missing_docs)]

//! WebAssembly export surface for Panduck.

use wasm_bindgen::prelude::*;

/// Format adapters wired into this artifact.
///
/// The WebAssembly binding currently exports metadata only and has no import,
/// transform or export primitives, so it must not claim any format. When a
/// conversion surface lands here, derive this list from
/// `document_convert::{READERS, WRITERS}` instead of a hand-written list.
const FORMATS: &[&str] = &[];

/// Returns the Panduck WASM binding version.
#[wasm_bindgen(js_name = panduckVersion)]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists format adapters wired into this workspace.
#[wasm_bindgen(js_name = supportedFormats)]
pub fn supported_formats() -> Vec<JsValue> {
    FORMATS.iter().map(|name| JsValue::from_str(name)).collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[wasm_bindgen(js_name = isSupportedFormat)]
pub fn is_supported_format(name: &str) -> bool {
    FORMATS.iter().any(|format| format.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    #[test]
    fn wasm_artifact_claims_no_formats() {
        assert!(super::FORMATS.is_empty(), "the metadata-only wasm artifact must not advertise formats");
    }
}
