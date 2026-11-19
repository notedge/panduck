use document_formats::import::docx::xml_extensions::{
    child_element, document_root, element_is, elements_by_local_name, elements_by_xpath, parse_xml_bytes,
};
use oak_xml::ast::XmlValue;

#[test]
fn xpath_collects_descendant_elements_by_local_name() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl>
  </w:abstractNum>
</w:numbering>"#;
    let value = parse_xml_bytes(xml).expect("parse");
    let root = document_root(&value).expect("root");
    let matches = elements_by_local_name(root, "abstractNum").expect("query");
    assert_eq!(matches.len(), 1);
    let levels = elements_by_xpath(matches[0], "//lvl").expect("xpath");
    assert_eq!(levels.len(), 1);
}

#[test]
fn parses_wordprocessingml_footnote_reference_run() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>See</w:t></w:r>
      <w:r><w:footnoteReference w:id="1"/></w:r>
      <w:r><w:t> for details.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let value = parse_xml_bytes(xml).expect("parse");
    let root = document_root(&value).expect("root");
    let body = child_element(root, "body").expect("body");
    let paragraph = child_element(body, "p").expect("paragraph");
    let runs = paragraph.children.iter().filter_map(XmlValue::as_element).filter(|child| element_is(child, "r")).collect::<Vec<_>>();
    assert_eq!(runs.len(), 3);
    let footnote_run = runs[1];
    let names = footnote_run.children.iter().filter_map(XmlValue::as_element).map(|child| child.name.clone()).collect::<Vec<_>>();
    assert!(child_element(footnote_run, "footnoteReference").is_some(), "expected footnoteReference child, got {names:?}");
}

#[test]
fn parses_wordprocessingml_run_properties() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r>
      <w:r><w:rPr><w:i/></w:rPr><w:t> italic</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let value = parse_xml_bytes(xml).expect("parse");
    let root = document_root(&value).expect("root");
    let body = child_element(root, "body").expect("body");
    let paragraph = child_element(body, "p").expect("paragraph");
    let runs = paragraph.children.iter().filter_map(XmlValue::as_element).filter(|child| element_is(child, "r")).collect::<Vec<_>>();
    assert_eq!(runs.len(), 2);
    let italic_r_pr = child_element(runs[1], "rPr").expect("rPr");
    assert!(
        child_element(italic_r_pr, "i").is_some(),
        "expected w:i in rPr, names: {:?}",
        italic_r_pr.children.iter().filter_map(XmlValue::as_element).map(|child| child.name.clone()).collect::<Vec<_>>()
    );
}
