use document_formats::import::epub::{ast_lowering::blocks_from_html_document, html_extensions::parse_html_bytes};
use document_ir::{Block, DocumentGraph, DocumentId, Inline, TableRow};

fn tables(graph: &DocumentGraph) -> Vec<&Block> {
    graph.blocks.iter().map(|node| &node.block).filter(|block| matches!(block, Block::Table { .. })).collect()
}

fn top_level_table(blocks: &[Block]) -> (&[TableRow], &[TableRow]) {
    blocks
        .iter()
        .find_map(|block| match block {
            Block::Table { head, body, .. } => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("table expected, got {blocks:?}"))
}

#[test]
fn body_selector_scopes_block_lowering() {
    const XHTML: &str = r#"<html><head><title>Skip</title></head><body><p>Body</p></body></html>"#;
    let document = parse_html_bytes(XHTML.as_bytes()).expect("parse");
    let mut graph = DocumentGraph::new(DocumentId(0));
    let (blocks, _) = blocks_from_html_document(&document, &mut graph, None);
    assert_eq!(blocks.len(), 1);
    assert!(matches!(blocks[0], Block::Paragraph { .. }));
}

#[test]
fn unnamed_spans_splice_their_children_instead_of_inventing_a_style() {
    const XHTML: &str = r#"<html><body><p>a<span>b<span>c</span></span>d <span class="lead">marked</span></p></body></html>"#;
    let document = parse_html_bytes(XHTML.as_bytes()).expect("parse");
    let mut graph = DocumentGraph::new(DocumentId(0));
    let (blocks, _) = blocks_from_html_document(&document, &mut graph, None);
    let Block::Paragraph { content } = &blocks[0]
    else {
        panic!("paragraph expected: {blocks:?}")
    };

    assert!(
        !content.iter().any(|inline| matches!(inline, Inline::Styled { style, .. } if style == "span")),
        "an unnamed span must not become a `span` style: {content:?}"
    );
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "abcd marked");

    let Inline::Attributed { attributes, children } = content.last().expect("attributed span")
    else {
        panic!("an authored span attribute must keep an attributed wrapper: {content:?}")
    };
    assert_eq!(attributes.classes, ["lead"]);
    assert_eq!(children, &[Inline::Text { text: "marked".into() }]);
}

#[test]
fn table_cell_keeps_block_level_children() {
    const XHTML: &str = r#"<html><body>
<table>
<thead><tr><th>head</th></tr></thead>
<tbody><tr><td><p>one</p><p>two</p><ul><li>item</li></ul></td></tr></tbody>
</table>
</body></html>"#;
    let document = parse_html_bytes(XHTML.as_bytes()).expect("parse");
    let mut graph = DocumentGraph::new(DocumentId(0));
    let (blocks, losses) = blocks_from_html_document(&document, &mut graph, None);
    assert!(losses.is_empty(), "block cell content must not record a loss: {losses:?}");
    let (head, body) = top_level_table(&blocks);
    assert_eq!(head.len(), 1);
    assert_eq!(body.len(), 1);
    let cell = &body[0].cells[0];

    let children: Vec<&Block> = cell.children.iter().filter_map(|id| graph.block(*id)).map(|node| &node.block).collect();
    assert_eq!(children.len(), 3, "one child block per authored cell block: {children:?}");
    assert!(matches!(children[0], Block::Paragraph { .. }), "{children:?}");
    assert!(matches!(children[1], Block::Paragraph { .. }), "{children:?}");
    assert!(matches!(children[2], Block::List { ordered: false, .. }), "{children:?}");
}

#[test]
fn nested_table_rows_stay_out_of_the_outer_table() {
    const XHTML: &str = r#"<html><body>
<table>
<tbody>
<tr><td><table><tbody><tr><td>nested</td></tr></tbody></table></td></tr>
</tbody>
</table>
</body></html>"#;
    let document = parse_html_bytes(XHTML.as_bytes()).expect("parse");
    let mut graph = DocumentGraph::new(DocumentId(0));
    let (blocks, _) = blocks_from_html_document(&document, &mut graph, None);

    assert_eq!(blocks.len(), 1, "the nested table must stay inside its cell, not become a sibling: {blocks:?}");
    let (head, body) = top_level_table(&blocks);
    assert!(head.is_empty(), "the outer table has no header: {head:?}");
    assert_eq!(body.len(), 1, "the outer table has exactly one authored row: {body:?}");
    let cell = &body[0].cells[0];
    assert_eq!(cell.children.len(), 1, "the cell carries the nested table as a child block: {cell:?}");

    let nested = cell.children[0];
    let Some(nested_node) = graph.block(nested)
    else {
        panic!("nested table node is missing")
    };
    let Block::Table { body: nested_body, .. } = &nested_node.block
    else {
        panic!("nested table block expected: {:?}", nested_node.block)
    };
    assert_eq!(nested_body.len(), 1, "nested table keeps its own row: {nested_body:?}");
    assert_eq!(
        tables(&graph).len(),
        1,
        "the only table node in the graph is the nested cell child, the outer table is a top-level block: {:?}",
        graph.blocks
    );
}
