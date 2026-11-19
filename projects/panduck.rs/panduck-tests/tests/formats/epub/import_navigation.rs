use document_formats::import::epub::navigation::parse_nav_toc;

#[test]
fn parses_nested_nav_entries() {
    const NAV: &str = r#"<nav epub:type="toc">
<ol>
  <li><a href="part.xhtml">Part One</a>
    <ol>
      <li><a href="part.xhtml#ch1">Chapter 1</a></li>
    </ol>
  </li>
</ol>
</nav>"#;
    let entries = parse_nav_toc(NAV.as_bytes()).expect("parse nav");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].label, "Part One");
    assert_eq!(entries[0].href, "part.xhtml");
    assert_eq!(entries[0].children.len(), 1);
    assert_eq!(entries[0].children[0].label, "Chapter 1");
    assert_eq!(entries[0].children[0].href, "part.xhtml#ch1");
}

#[test]
fn selects_toc_nav_via_css_selector() {
    const NAV: &str = r#"<body>
<nav id="other"><a href="skip.xhtml">Skip</a></nav>
<nav epub:type="toc"><ol><li><a href="part.xhtml">Part</a></li></ol></nav>
</body>"#;
    let entries = parse_nav_toc(NAV.as_bytes()).expect("parse nav");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].href, "part.xhtml");
}
