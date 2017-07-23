use panduck_napi::{is_supported_format, panduck_version, supported_formats};

#[test]
fn version_is_non_empty() {
    assert!(!panduck_version().is_empty());
}

#[test]
fn formats_include_markdown() {
    let formats = supported_formats();
    assert!(formats.iter().any(|name| name == "markdown"));
    assert!(is_supported_format("markdown".to_string()));
}
