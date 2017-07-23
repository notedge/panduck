//! Node-API export surface for Panduck.

use napi_derive::napi;

const FORMATS: [&str; 4] = ["markdown", "org", "rst", "tex"];

/// Returns the Panduck N-API binding version.
#[napi]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists format adapters wired into this workspace.
#[napi]
pub fn supported_formats() -> Vec<String> {
    FORMATS.iter().map(|name| (*name).to_string()).collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[napi]
pub fn is_supported_format(name: String) -> bool {
    FORMATS.iter().any(|format| format.eq_ignore_ascii_case(name.as_str()))
}
