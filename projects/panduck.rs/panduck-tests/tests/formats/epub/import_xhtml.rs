use document_formats::{
    export::markdown::export_markdown,
    import::epub::xhtml::{blocks_from_xhtml, blocks_from_xhtml_with_context},
};
use document_ir::{Block, DocumentGraph, DocumentId};

const SAMPLE_XHTML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter One</h1>
    <p>Hello <strong>EPUB</strong></p>
  </body>
</html>"#;

#[test]
fn oak_html_lowers_headings_paragraphs_and_inline_styles() {
    let blocks = blocks_from_xhtml(SAMPLE_XHTML.as_bytes()).expect("lower xhtml");
    assert!(blocks.len() >= 2, "blocks={blocks:?}");
    let mut graph = DocumentGraph::new(DocumentId(1));
    for block in blocks {
        graph.push_block(block);
    }
    let exported = export_markdown(&graph).expect("export");
    assert!(exported.contains("# Chapter One"));
    assert!(exported.contains("**EPUB**"));
}

#[test]
fn oak_html_lowers_tables_and_nested_lists() {
    const XHTML: &str = r#"<html><body>
<table><tr><th>H</th><td>V</td></tr></table>
<ul><li>Top<ul><li>Nested</li></ul></li></ul>
</body></html>"#;
    let mut graph = DocumentGraph::new(DocumentId(2));
    let blocks = blocks_from_xhtml_with_context(XHTML.as_bytes(), &mut graph, None).expect("lower xhtml");
    assert!(blocks.iter().any(|block| matches!(block, Block::Table { .. })), "blocks={blocks:?}");
    for block in blocks {
        graph.push_block(block);
    }
    let exported = export_markdown(&graph).expect("export");
    assert!(exported.contains("| H | V |"));
    assert!(exported.contains("- Top"));
    assert!(exported.contains("  - Nested"));
}

#[test]
fn css_selector_registers_spine_images() {
    const XHTML: &str = r#"<html><body>
<p><img src="images/cover.png" alt="Cover"/></p>
<svg><image xlink:href="images/diagram.svg"/></svg>
</body></html>"#;
    let mut graph = DocumentGraph::new(DocumentId(3));
    let blocks = blocks_from_xhtml_with_context(XHTML.as_bytes(), &mut graph, Some("OEBPS/chapter.xhtml")).expect("lower xhtml");
    assert!(!blocks.is_empty());
    assert_eq!(graph.assets.len(), 2);
    assert!(graph.assets.iter().any(|asset| asset.source.as_deref() == Some("OEBPS/images/cover.png")));
    assert!(graph.assets.iter().any(|asset| asset.source.as_deref() == Some("OEBPS/images/diagram.svg")));
}

#[test]
fn ast_lowering_handles_blockquote_pre_and_hr_without_body_wrapper() {
    const XHTML: &str = r#"<blockquote><p>Quoted line</p></blockquote>
<pre><code class="language-rust">fn main() {}</code></pre><hr/>"#;
    let blocks = blocks_from_xhtml(XHTML.as_bytes()).expect("lower xhtml");
    assert!(blocks.iter().any(|block| matches!(block, Block::Quote { .. })), "blocks={blocks:?}");
    assert!(
        blocks.iter().any(|block| matches!(
            block,
            Block::Code { content, .. } if content.contains("fn main")
        )),
        "blocks={blocks:?}"
    );
    assert!(
        blocks.iter().any(|block| matches!(
            block,
            Block::Opaque { kind, .. } if kind == "thematic_break"
        )),
        "blocks={blocks:?}"
    );
}

#[test]
fn oak_html_lowers_container_blockquote_code_and_hr() {
    const XHTML: &str = r#"<html><body>
<section><div><p>Wrapped</p></div></section>
<blockquote><p>Quote</p></blockquote>
<pre><code class="language-js">console.log(1)</code></pre>
<hr/>
</body></html>"#;
    let mut graph = DocumentGraph::new(DocumentId(3));
    let blocks = blocks_from_xhtml_with_context(XHTML.as_bytes(), &mut graph, None).expect("lower xhtml");
    for block in blocks {
        graph.push_block(block);
    }
    let exported = export_markdown(&graph).expect("export");
    assert!(exported.contains("Wrapped"));
    assert!(exported.contains("> Quote"));
    assert!(exported.contains("```js"));
    assert!(exported.contains("console.log(1)"));
    assert!(exported.contains("---"));
}
