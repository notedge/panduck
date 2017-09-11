use diagnostic::DiagnosticSeverity;
use notedown_ir::{DocumentGraph, IdAllocator, LossMarker, SemanticStatus};
use panduck_diagnostic::{
    diagnostics_from_graph, from_adapter_error, from_loss_marker, DiagnosticCode,
};
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
    assert_eq!(
        diagnostic.code().as_str(),
        "reader.docx.numbering_unresolved"
    );
    assert_eq!(diagnostic.severity(), DiagnosticSeverity::Warning);
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
    assert_eq!(
        set.diagnostics()[0].code().as_str(),
        "panduck.writer.unsupported-feature"
    );
}
