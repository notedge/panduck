//! Selector budget, namespace, `xml:space` and malformed-input gates.

use document_formats::{
    FormatError, QueryLimits,
    import::docx::xml_extensions::{
        document_root, element_text, elements_by_local_name, elements_by_xpath, elements_by_xpath_with_limits, parse_xml_bytes,
        xml_space_preserved,
    },
};

fn root(value: &oak_xml::ast::XmlValue) -> &oak_xml::ast::XmlElement {
    document_root(value).expect("document root")
}

#[test]
fn selector_budget_exhaustion_is_reported_instead_of_truncated() {
    let text = "<root><p/><p/><p/><p/><p/></root>";
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    let limits = QueryLimits { max_matches: 2, ..QueryLimits::default() };
    let error = elements_by_xpath_with_limits(element, "//p", limits).expect_err("budget stops the query");
    assert!(matches!(error, FormatError::Budget { .. }), "{error:?}");
}

#[test]
fn selector_budget_below_match_count_completes() {
    let text = "<root><p/><p/><p/></root>";
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    let limits = QueryLimits { max_matches: 8, ..QueryLimits::default() };
    let matches = elements_by_xpath_with_limits(element, "//p", limits).expect("query completes");
    assert_eq!(matches.len(), 3);
}

#[test]
fn namespace_prefixes_match_textually() {
    let text = r#"<w:document xmlns:w="urn:word" xmlns:x="urn:other"><w:p/><x:p/><p/></w:document>"#;
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    assert_eq!(elements_by_xpath(element, "//w:p").expect("query").len(), 1);
    assert_eq!(elements_by_xpath(element, "//x:p").expect("query").len(), 1);
    assert_eq!(elements_by_xpath(element, "//p").expect("query").len(), 3);
    assert_eq!(elements_by_local_name(element, "p").expect("query").len(), 3);
}

#[test]
fn unknown_namespace_prefix_matches_nothing() {
    let text = r#"<w:document xmlns:w="urn:word"><w:p/></w:document>"#;
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    assert!(elements_by_xpath(element, "//z:p").expect("query").is_empty());
}

#[test]
fn malformed_xml_is_rejected_before_selector_execution() {
    let error = parse_xml_bytes(b"<root><p></root>").expect_err("unclosed element");
    assert!(matches!(error, FormatError::Parse { .. }), "{error:?}");
}

#[test]
fn xml_space_preserve_is_detected_and_text_kept_verbatim() {
    let text = r#"<w:t xmlns:w="urn:word" xml:space="preserve">  a  b  </w:t>"#;
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    assert!(xml_space_preserved(element));
    assert_eq!(element_text(element), "  a  b  ");
}

#[test]
fn text_without_xml_space_preserve_is_also_kept_verbatim() {
    let text = r#"<w:t xmlns:w="urn:word">  a  b  </w:t>"#;
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    assert!(!xml_space_preserved(element));
    assert_eq!(element_text(element), "  a  b  ");
}

#[test]
fn entities_inside_preserved_text_still_decode() {
    let text = r#"<w:t xmlns:w="urn:word" xml:space="preserve"> a &amp; b &#x3F; </w:t>"#;
    let value = parse_xml_bytes(text.as_bytes()).expect("parse xml");
    let element = root(&value);
    assert_eq!(element_text(element), " a & b ? ");
}
