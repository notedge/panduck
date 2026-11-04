//! Smoke contract for the Node-API route surface.
//!
//! Behavioural conversion coverage lives in `projects/panduck.ts/packages/panduck/tests`.
//! This test only proves the adapter is wired and rejects unknown targets.

#[test]
fn supported_conversion_accepts_docx_to_markdown() {
    assert!(panduck_napi::supported_conversion("docx".into(), "markdown".into()));
}

#[test]
fn supported_conversion_rejects_unknown_target() {
    assert!(!panduck_napi::supported_conversion("docx".into(), "not-a-format".into()));
}
