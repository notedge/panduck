//! Provenance sidecar isolation — source metadata must not affect block
//! semantics or export.

use document_formats::{export::markdown::export_markdown, import::markdown::import_markdown_bytes};
use document_ir::{Block, DocumentGraph, SemanticStatus, SourceKind, SourceRef};

fn block_semantics(graph: &DocumentGraph) -> Vec<&Block> {
    graph.blocks.iter().map(|node| &node.block).collect()
}

#[test]
fn source_sidecar_does_not_affect_markdown_export() {
    let source_md = "# Title\n\nShared body text.\n";
    let mut with_text_span = import_markdown_bytes("text-span.md", source_md).expect("import markdown");
    let mut with_package_source = import_markdown_bytes("package.md", source_md).expect("import markdown");

    assert_eq!(block_semantics(&with_text_span), block_semantics(&with_package_source));

    let text_span_node = with_text_span.blocks.iter().find(|node| matches!(node.block, Block::Paragraph { .. })).expect("paragraph").id;
    with_text_span.attach_source(SourceRef {
        node: text_span_node,
        kind: SourceKind::TextSpan { start: 12, end: 28 },
        precision: SemanticStatus::Resolved,
    });

    let package_node = with_package_source.blocks.iter().find(|node| matches!(node.block, Block::Paragraph { .. })).expect("paragraph").id;
    with_package_source.attach_source(SourceRef {
        node: package_node,
        kind: SourceKind::PackageMember { path: "word/document.xml".to_string() },
        precision: SemanticStatus::Resolved,
    });
    with_package_source.attach_source(SourceRef {
        node: package_node,
        kind: SourceKind::Synthetic { reason: "merged w:r".to_string() },
        precision: SemanticStatus::Inferred,
    });

    let markdown_from_text_span = export_markdown(&with_text_span).expect("export text-span graph");
    let markdown_from_package = export_markdown(&with_package_source).expect("export package graph");
    assert_eq!(markdown_from_text_span, markdown_from_package);

    let json_text_span: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&with_text_span).expect("serialize text-span")).expect("parse json");
    let json_package: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&with_package_source).expect("serialize package")).expect("parse json");

    assert_ne!(json_text_span["sources"], json_package["sources"]);
    assert_eq!(json_text_span["blocks"], json_package["blocks"]);
    assert!(json_text_span["blocks"].to_string().contains("Shared body text."));
    assert!(!json_text_span["blocks"].to_string().contains("TextSpan"));
    assert!(!json_text_span["blocks"].to_string().contains("\"start\""));
}

#[test]
fn unsupported_source_precision_round_trips_in_sidecar_only() {
    let mut graph = import_markdown_bytes("probe.md", "Probe paragraph.\n").expect("import");
    let paragraph = graph.blocks[0].id;
    graph.attach_source(SourceRef {
        node: paragraph,
        kind: SourceKind::Synthetic { reason: "import precision unknown".to_string() },
        precision: SemanticStatus::Unsupported,
    });

    let encoded = serde_json::to_string(&graph).expect("serialize");
    let restored: DocumentGraph = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(restored.sources.len(), 1);
    assert_eq!(restored.sources[0].precision, SemanticStatus::Unsupported);
    assert_eq!(restored.block(paragraph).expect("paragraph").block, graph.block(paragraph).expect("paragraph").block);
}
