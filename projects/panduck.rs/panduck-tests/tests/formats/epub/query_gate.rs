//! CSS selector budget and unsupported-selector gates.

use document_formats::{
    FormatError, QueryLimits,
    import::epub::html_extensions::{parse_html_text, select_css_with_limits},
};

#[test]
fn css_selector_budget_exhaustion_is_reported() {
    let document = parse_html_text("<html><body><p>a</p><p>b</p><p>c</p></body></html>").expect("parse html");
    let limits = QueryLimits { max_matches: 2, ..QueryLimits::default() };
    let error = select_css_with_limits(&document, "p", "epub", limits).expect_err("budget stops the query");
    assert!(matches!(error, FormatError::Budget { .. }), "{error:?}");
}

#[test]
fn css_selector_below_budget_completes() {
    let document = parse_html_text("<html><body><p>a</p><p>b</p></body></html>").expect("parse html");
    let limits = QueryLimits { max_matches: 8, ..QueryLimits::default() };
    let matches = select_css_with_limits(&document, "p", "epub", limits).expect("query completes");
    assert_eq!(matches.len(), 2);
}
