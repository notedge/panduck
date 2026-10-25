use boost_diagnostic::DiagnosticSeverity;
use notedown_ir::{DocumentGraph, IdAllocator, LossMarker, SemanticStatus};
use panduck_diagnostic::{DiagnosticEnvelope, diagnostics_from_graph, from_adapter_error, from_loss_marker};
use panduck_types::AdapterError;

#[test]
fn adapter_error_maps_to_dotted_wire_code() {
    let diagnostic = from_adapter_error(&AdapterError::not_implemented("docx writer"));
    assert_eq!(diagnostic.code().as_str(), "panduck.adapter.not-implemented");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Error);
}

#[test]
fn loss_marker_maps_to_semantic_diagnostic() {
    let marker = LossMarker {
        code: "reader.docx.numbering_unresolved".to_string(),
        message: "numbering.xml missing".to_string(),
        status: SemanticStatus::Partial,
    };
    let diagnostic = from_loss_marker(&marker);
    assert_eq!(diagnostic.code().as_str(), "reader.docx.numbering_unresolved");
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Warning);
}

#[test]
fn loss_marker_status_arg_uses_controlled_wire_label() {
    let marker = LossMarker {
        code: "reader.docx.numbering_unresolved".to_string(),
        message: "numbering.xml missing".to_string(),
        status: SemanticStatus::Partial,
    };
    let set = {
        let mut set = boost_diagnostic::DiagnosticSet::new();
        set.push(from_loss_marker(&marker));
        set
    };
    let json = serde_json::to_string(&DiagnosticEnvelope::from_set(&set))
        .expect("serialize diagnostic envelope");
    assert!(json.contains("\"severity\""));
    assert!(json.contains("\"partial\""), "status arg must use the controlled label, got {json}");
    assert!(!json.contains("\"Partial\""), "Debug-formatted status must not leak, got {json}");
}

#[test]
fn graph_losses_collect_into_diagnostic_set() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_loss(LossMarker {
        code: "panduck.writer.unsupported-feature".to_string(),
        message: "strike not expressible".to_string(),
        status: SemanticStatus::Lossy,
    });
    let set = diagnostics_from_graph(&graph);
    assert_eq!(set.diagnostics().len(), 1);
    assert_eq!(set.diagnostics()[0].code().as_str(), "panduck.writer.unsupported-feature");
}

#[test]
fn diagnostic_envelope_serializes_to_json() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_loss(LossMarker {
        code: "import.epub.partial_coverage".to_string(),
        message: "partial EPUB coverage".to_string(),
        status: SemanticStatus::Partial,
    });
    let set = diagnostics_from_graph(&graph);
    let envelope = DiagnosticEnvelope::from_set(&set);
    let json = serde_json::to_string(&envelope).expect("serialize diagnostic envelope");
    assert!(json.contains("import.epub.partial_coverage"));
}

#[test]
fn adapter_error_typed_args_in_json_envelope() {
    let diag = from_adapter_error(&AdapterError::invalid_range(42usize, 7usize));
    let set = {
        let mut s = boost_diagnostic::DiagnosticSet::new();
        s.push(diag);
        s
    };
    let envelope = DiagnosticEnvelope::from_set(&set);
    let json = serde_json::to_string(&envelope).expect("serialize adapter-error envelope");
    // 验证 MessageArg::U64 的 typed args 序列化正确 (external tag variant name as key)
    assert!(json.contains("\"offset\""));
    assert!(json.contains("42"));
    assert!(json.contains("\"len\""));
    assert!(json.contains("7"));
    assert!(json.contains("panduck.adapter.invalid-range"));
}

#[test]
fn adapter_error_text_args_in_json_envelope() {
    let diag = from_adapter_error(&AdapterError::adapter("docx-writer", "numbering unresolved"));
    let set = {
        let mut s = boost_diagnostic::DiagnosticSet::new();
        s.push(diag);
        s
    };
    let envelope = DiagnosticEnvelope::from_set(&set);
    let json = serde_json::to_string(&envelope).expect("serialize adapter envelope");
    assert!(json.contains("\"adapter\""));
    assert!(json.contains("docx-writer"));
    assert!(json.contains("\"message\""));
    assert!(json.contains("numbering unresolved"));
    assert!(json.contains("panduck.adapter.failure"));
}
