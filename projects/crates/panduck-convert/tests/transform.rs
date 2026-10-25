use notedown_formats::import::html::import_html_bytes;
use panduck_convert::{apply_transform_passes, convert_bytes};
use serde_json::Value;

mod common;
use common::assert_convert_report;

fn plan_json(transforms: &str) -> String {
    format!(
        r#"{{"version":"panduck-conversion-v0","id":"test-plan","input":{{"format":"html"}},"transforms":[{transforms}],"output":{{"format":"markdown"}},"publish":{{"kind":"none"}}}}"#
    )
}

fn report(output: &panduck_convert::ConvertOutput) -> Value {
    serde_json::from_str(&output.report_json).expect("parse report")
}

fn diagnostic_codes(report: &Value) -> Vec<String> {
    report["diagnostics"]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(ToString::to_string))
        .collect()
}

#[test]
fn replace_text_pass_rewrites_heading_and_paragraph() {
    let plan = plan_json(
        r#"{"name":"replace-text","description":"rewrite greeting","params":{"search":"Hello","replacement":"Bonjour"}}"#,
    );
    let output = convert_bytes(
        "html",
        "markdown",
        "greeting.html",
        br#"<html><body><h1>Hello World</h1><p>Hello there.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with replace-text");

    assert_convert_report(&output, "markdown");
    assert!(output.markdown.contains("Bonjour World"), "markdown was: {}", output.markdown);
    assert!(output.markdown.contains("Bonjour there."), "markdown was: {}", output.markdown);
    assert!(!output.markdown.contains("Hello"));

    let report = report(&output);
    let transform = &report["pipeline"]["transforms"][0];
    assert_eq!(transform["name"], "replace-text");
    assert_eq!(transform["status"], "applied");
    assert_eq!(transform["changedNodes"], 2);
    assert_eq!(transform["replacedOccurrences"], 2);
    assert_eq!(transform["description"], "rewrite greeting");
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.pass-applied"));
}

#[test]
fn remove_blocks_pass_deletes_matching_block_kind() {
    let plan = plan_json(r#"{"name":"remove-blocks","targetKinds":["quote"]}"#);
    let output = convert_bytes(
        "html",
        "markdown",
        "quote.html",
        br#"<html><body><blockquote>Quoted text.</blockquote><p>Keep me.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with remove-blocks");

    assert_convert_report(&output, "markdown");
    assert!(!output.markdown.contains("Quoted text."), "markdown was: {}", output.markdown);
    assert!(output.markdown.contains("Keep me."));

    let report = report(&output);
    assert_eq!(report["pipeline"]["transforms"][0]["status"], "applied");
    assert_eq!(report["pipeline"]["transforms"][0]["changedNodes"], 1);
    assert_eq!(report["pipeline"]["transforms"][0]["targetKinds"][0], "quote");
}

#[test]
fn replace_text_keeps_diagram_source_and_remove_blocks_targets_diagrams() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let diagram = graph
        .push_block(Block::Diagram { language: "mermaid".to_string(), source: "flowchart TD\n  Hello --> World".to_string() });
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello body.".to_string() }] });

    let replace = plan_json(r#"{"name":"replace-text","params":{"search":"Hello","replacement":"Bonjour"}}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&replace));
    assert_eq!(outcome.passes[0].changed_nodes, 1, "only the paragraph is rewritten");

    let Block::Diagram { language, source } = &graph.block(diagram).expect("diagram remains").block
    else {
        panic!("diagram block expected")
    };
    assert_eq!(language, "mermaid");
    assert!(source.contains("Hello"), "diagram source is not prose: {source}");

    let remove = plan_json(r#"{"name":"remove-blocks","targetKinds":["diagram"]}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&remove));
    assert_eq!(outcome.passes[0].changed_nodes, 1);
    assert!(graph.block(diagram).is_none());
}

#[test]
fn remove_blocks_pass_sweeps_descendants_of_removed_section() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Nested body.".to_string() }] });
    let section = graph.push_block(Block::Section {
        level: 1,
        title: vec![Inline::Text { text: "Section title".to_string() }],
        children: vec![paragraph],
    });
    let plan = plan_json(r#"{"name":"remove-blocks","targetKinds":["section"]}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));

    assert_eq!(outcome.passes.len(), 1);
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 1);
    assert_eq!(outcome.passes[0].swept_nodes, Some(1));
    assert!(graph.block(section).is_none());
    assert!(graph.block(paragraph).is_none());

    let markdown = notedown_formats::export::markdown::export_markdown(&graph).expect("export after sweep");
    assert!(!markdown.contains("Section title"), "markdown was: {markdown}");
    assert!(!markdown.contains("Nested body."), "markdown was: {markdown}");
}

#[test]
fn unknown_pass_name_is_reported_and_skipped() {
    let plan = plan_json(r#"{"name":"no-such-pass"}"#);
    let output = convert_bytes(
        "html",
        "markdown",
        "unsupported.html",
        br#"<html><body><p>Unchanged body.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with unsupported pass");

    assert_convert_report(&output, "markdown");
    assert!(output.markdown.contains("Unchanged body."));

    let report = report(&output);
    let transform = &report["pipeline"]["transforms"][0];
    assert_eq!(transform["name"], "no-such-pass");
    assert_eq!(transform["status"], "unsupported");
    assert_eq!(transform["changedNodes"], 0);
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.unsupported-pass"));
}

#[test]
fn resolve_references_is_a_supported_pass_without_references() {
    let plan = plan_json(r#"{"name":"resolve-references"}"#);
    let output = convert_bytes(
        "html",
        "markdown",
        "no-references.html",
        br#"<html><body><p>Plain body.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with resolve-references");

    assert_convert_report(&output, "markdown");
    assert!(output.markdown.contains("Plain body."));

    let report = report(&output);
    let transform = &report["pipeline"]["transforms"][0];
    assert_eq!(transform["name"], "resolve-references");
    assert_eq!(transform["status"], "skipped");
    assert_eq!(transform["changedNodes"], 0);
    assert_eq!(transform["replacedOccurrences"], 0);
    let codes = diagnostic_codes(&report);
    assert!(!codes.iter().any(|code| code == "panduck.transform.unsupported-pass"), "codes were: {codes:?}");
}

fn section_block(level: u8, title: &str, children: Vec<notedown_ir::NodeId>) -> notedown_ir::Block {
    notedown_ir::Block::Section { level, title: vec![notedown_ir::Inline::Text { text: title.to_string() }], children }
}

fn section_level(graph: &notedown_ir::DocumentGraph, id: notedown_ir::NodeId) -> u8 {
    match &graph.block(id).expect("block exists").block {
        notedown_ir::Block::Section { level, .. } => *level,
        other => panic!("expected section, got {other:?}"),
    }
}

#[test]
fn normalize_headings_renumbers_nested_sections_and_is_idempotent() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Body.".to_string() }] });
    let grandchild = graph.push_block(section_block(6, "Grandchild", Vec::new()));
    let child = graph.push_block(section_block(5, "Child", vec![grandchild]));
    let root = graph.push_block(section_block(3, "Root", vec![paragraph, child]));
    let sibling = graph.push_block(section_block(1, "Sibling", Vec::new()));

    let plan = plan_json(r#"{"name":"normalize-headings"}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));

    assert_eq!(outcome.passes.len(), 1);
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 3);
    assert!(outcome.diagnostics.iter().any(|diagnostic| diagnostic.code().as_str() == "panduck.transform.pass-applied"));
    assert_eq!(section_level(&graph, root), 1);
    assert_eq!(section_level(&graph, child), 2);
    assert_eq!(section_level(&graph, grandchild), 3);
    assert_eq!(section_level(&graph, sibling), 1, "already-normalized sibling stays at level 1");

    let second = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(second.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(second.passes[0].changed_nodes, 0);
    assert!(second.diagnostics.is_empty(), "idempotent rerun must not emit diagnostics");
}

#[test]
fn normalize_headings_clamps_deep_levels_at_six() {
    use notedown_ir::{DocumentGraph, DocumentId};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let mut ids = Vec::new();
    let mut current = graph.push_block(section_block(9, "Level 8", Vec::new()));
    ids.push(current);
    for index in (1..8).rev() {
        current = graph.push_block(section_block(9, &format!("Level {index}"), vec![current]));
        ids.push(current);
    }

    let plan = plan_json(r#"{"name":"normalize-headings"}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 8);

    for (index, id) in ids.iter().enumerate() {
        let expected = (ids.len() - index).min(6) as u8;
        assert_eq!(section_level(&graph, *id), expected, "section at depth index {index}");
    }
}

#[test]
fn resolve_references_fills_empty_display_from_target_text_and_is_idempotent() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline, RelationEndpoint};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let body = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Target body".to_string() }] });
    let heading = graph.push_block(section_block(1, "Section title", Vec::new()));
    let holder = graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Styled {
                style: "em".to_string(),
                children: vec![Inline::Reference { target: body, display: String::new() }],
            },
            Inline::Link {
                target: RelationEndpoint::Document(DocumentId(1)),
                display: vec![Inline::Reference { target: heading, display: String::new() }],
                title: None,
            },
        ],
    });

    let plan = plan_json(r#"{"name":"resolve-references"}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 1);
    assert_eq!(outcome.passes[0].replaced_occurrences, Some(2));
    assert!(outcome.diagnostics.iter().any(|diagnostic| diagnostic.code().as_str() == "panduck.transform.pass-applied"));

    let Block::Paragraph { content } = &graph.block(holder).expect("holder exists").block
    else {
        panic!("paragraph expected")
    };
    let Inline::Styled { children, .. } = &content[0]
    else {
        panic!("styled inline expected")
    };
    assert_eq!(children, &[Inline::Reference { target: body, display: "Target body".to_string() }]);
    let Inline::Link { display, .. } = &content[1]
    else {
        panic!("link inline expected")
    };
    assert_eq!(display, &[Inline::Reference { target: heading, display: "Section title".to_string() }]);

    let second = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(second.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(second.passes[0].changed_nodes, 0);
    assert_eq!(second.passes[0].replaced_occurrences, Some(0));
    assert!(second.diagnostics.is_empty(), "idempotent rerun must not emit diagnostics");
}

#[test]
fn resolve_references_replaces_dangling_reference_and_warns() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline, NodeId};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let holder = graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Reference { target: NodeId(999), display: "Missing note".to_string() },
            Inline::Text { text: " then ".to_string() },
            Inline::Reference { target: NodeId(1000), display: String::new() },
        ],
    });

    let plan = plan_json(r#"{"name":"resolve-references"}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 1);
    assert_eq!(outcome.passes[0].replaced_occurrences, Some(2));

    let dangling: Vec<_> = outcome
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code().as_str() == "panduck.transform.dangling-reference")
        .collect();
    assert_eq!(dangling.len(), 2);
    assert!(dangling.iter().any(|diagnostic| diagnostic.message().to_string().contains("999")));
    assert!(dangling.iter().any(|diagnostic| diagnostic.message().to_string().contains("1000")));

    let Block::Paragraph { content } = &graph.block(holder).expect("holder exists").block
    else {
        panic!("paragraph expected")
    };
    assert_eq!(
        content,
        &[
            Inline::Text { text: "Missing note".to_string() },
            Inline::Text { text: " then ".to_string() },
            Inline::Text { text: String::new() },
        ]
    );

    let second = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(second.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(second.passes[0].changed_nodes, 0);
    assert!(second.diagnostics.is_empty(), "dangling references are consumed on the first pass");
}

#[test]
fn resolve_references_keeps_authored_display_text() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let target = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Target body".to_string() }] });
    let holder =
        graph.push_block(Block::Paragraph { content: vec![Inline::Reference { target, display: "Keep me".to_string() }] });

    let plan = plan_json(r#"{"name":"resolve-references"}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(outcome.passes[0].changed_nodes, 0);
    assert_eq!(outcome.passes[0].replaced_occurrences, Some(0));
    assert!(outcome.diagnostics.is_empty());

    let Block::Paragraph { content } = &graph.block(holder).expect("holder exists").block
    else {
        panic!("paragraph expected")
    };
    assert_eq!(content, &[Inline::Reference { target, display: "Keep me".to_string() }]);
}

#[test]
fn replace_text_without_search_is_reported_as_invalid_params() {
    let plan = plan_json(r#"{"name":"replace-text","params":{"replacement":"x"}}"#);
    let output =
        convert_bytes("html", "markdown", "invalid.html", br#"<html><body><p>Body.</p></body></html>"#.to_vec(), Some(&plan))
            .expect("convert with invalid replace-text params");

    assert_convert_report(&output, "markdown");
    let report = report(&output);
    assert_eq!(report["pipeline"]["transforms"][0]["status"], "invalid-params");
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.invalid-params"));
}

#[test]
fn conversion_without_transforms_keeps_pipeline_and_graph_unchanged() {
    let bytes = br#"<html><body><h1>Title</h1><p>Body.</p></body></html>"#.to_vec();
    let baseline = convert_bytes("html", "markdown", "plain.html", bytes.clone(), None).expect("baseline convert");
    let empty_plan =
        convert_bytes("html", "markdown", "plain.html", bytes, Some(&plan_json(""))).expect("convert with empty pass list");

    assert_eq!(baseline.markdown, empty_plan.markdown);
    let baseline_report = report(&baseline);
    assert!(baseline_report["pipeline"].get("transforms").is_none());
    let empty_report = report(&empty_plan);
    assert!(empty_report["pipeline"].get("transforms").is_none());
}

#[test]
fn apply_transform_passes_reports_skip_when_no_text_matches() {
    let mut graph = import_html_bytes("skip.html", br#"<p>No match here.</p>"#).expect("import html");
    let plan = plan_json(r#"{"name":"replace-text","params":{"search":"Absent","replacement":"x"}}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));

    assert_eq!(outcome.passes.len(), 1);
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(outcome.passes[0].changed_nodes, 0);
    assert!(outcome.diagnostics.is_empty());
}
