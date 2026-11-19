use document_formats::{FormatDirection, FormatStatus, capabilities};

#[test]
fn capabilities_do_not_overstate_partial_formats() {
    let caps = capabilities();
    #[cfg(feature = "notedown")]
    assert!(caps.iter().any(|cap| { cap.id == "notedown" && cap.direction == FormatDirection::Import && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "markdown")]
    assert!(caps.iter().any(|cap| { cap.id == "markdown" && cap.direction == FormatDirection::Import && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "markdown")]
    assert!(caps.iter().any(|cap| { cap.id == "markdown" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Partial }));
    // GFM is an `oak-markdown` profile, so it must never appear as a second
    // format id in the capability table.
    assert!(!caps.iter().any(|cap| cap.id.eq_ignore_ascii_case("gfm")));
    #[cfg(feature = "docx")]
    assert!(caps.iter().any(|cap| { cap.id == "docx" && cap.direction == FormatDirection::Import && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "docx")]
    assert!(caps.iter().any(|cap| { cap.id == "docx" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "pdf")]
    assert!(caps.iter().any(|cap| { cap.id == "pdf" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "epub")]
    assert!(caps.iter().any(|cap| { cap.id == "epub" && cap.direction == FormatDirection::Import && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "epub")]
    assert!(caps.iter().any(|cap| { cap.id == "epub" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "doc")]
    assert!(caps.iter().any(|cap| { cap.id == "doc" && cap.direction == FormatDirection::Import && cap.status == FormatStatus::Partial }));
    #[cfg(feature = "doc")]
    assert!(caps.iter().any(|cap| { cap.id == "doc" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Unavailable }));
    assert!(caps.iter().any(|cap| { cap.id == "html" && cap.direction == FormatDirection::Export && cap.status == FormatStatus::Partial }));
    #[cfg(not(feature = "docx"))]
    assert!(!caps.iter().any(|cap| cap.id == "docx"));
    #[cfg(not(feature = "epub"))]
    assert!(!caps.iter().any(|cap| cap.id == "epub"));
}

#[cfg(feature = "pandoc-json")]
#[test]
fn pandoc_json_declares_both_directions_as_partial() {
    let caps = capabilities();
    // The wire codec is complete for `[1,23,1]`, but the IR adapter projects
    // containers, complex tables and citations with recorded losses, so neither
    // direction is `Ready`.
    for direction in [FormatDirection::Import, FormatDirection::Export] {
        assert!(
            caps.iter().any(|cap| { cap.id == "pandoc-json" && cap.direction == direction && cap.status == FormatStatus::Partial }),
            "pandoc-json {direction:?}"
        );
    }
}
