//! End-to-end acceptance for the diagram chain the Pandark crawl pipeline
//! relies on: real-world HTML mermaid containers become `Block::Diagram`
//! without losing the authored source, and the DaVinci diagram subsystem
//! lowers that source into a renderable SVG scene.

use std::{collections::HashMap, fs, path::PathBuf};

use document_formats::{
    export::{
        html::{RenderedDiagram, export_html, export_html_with_diagrams},
        markdown::export_markdown,
    },
    import::{html::import_html_bytes, markdown::import_markdown_bytes},
};
use document_ir::{Block, DocumentGraph, NodeId};
use plotter_input_dot::dot_to_scene;
use plotter_input_mermaid::mermaid_to_scene;
use plotter_input_plantuml::plantuml_to_scene;

/// A docs page fragment shaped the way real sites embed diagrams: the Mermaid
/// runtime convention (`<div class="mermaid">`) and the static-site convention
/// (`<pre><code class="language-mermaid">`).
const PAGE: &str = r#"<!DOCTYPE html>
<html><head><title>Crawl pipeline notes</title></head><body>
<article>
<h1>Crawl pipeline</h1>
<p>The fetch to extract flow:</p>
<div class="mermaid">
flowchart TD
  A[Fetch page] --> B{Status OK?}
  B -->|yes| C[Extract blocks]
  B -->|no| D[Record failure]
</div>
<p>And the worker handshake:</p>
<pre><code class="language-mermaid">sequenceDiagram
  Frontier-&gt;&gt;Worker: claim
  Worker-&gt;&gt;Frontier: commit</code></pre>
</article>
</body></html>"#;

fn diagram_sources(label: &str) -> Vec<String> {
    let graph = import_html_bytes(label, PAGE.as_bytes()).expect("import html page");
    let sources = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Diagram { language, source } => {
                assert_eq!(language, "mermaid", "{label}: diagram language");
                Some(source.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(sources.len(), 2, "{label}: both mermaid containers must become diagrams: {:?}", graph.blocks);
    sources
}

#[test]
fn html_mermaid_containers_lower_to_renderable_svg_scenes() {
    let sources = diagram_sources("pipeline.html");
    assert!(sources[0].contains("flowchart TD"), "div container keeps the flowchart source:\n{}", sources[0]);
    assert!(sources[1].contains("sequenceDiagram"), "fenced code keeps the sequence source:\n{}", sources[1]);

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("diagram-artifacts");
    fs::create_dir_all(&out_dir).expect("create diagram artifact dir");

    let names = ["flowchart", "sequence"];
    let expected_labels: [&[&str]; 2] =
        [&["Fetch page", "Status OK?", "Extract blocks", "Record failure", "yes"], &["Frontier", "Worker", "claim", "commit"]];
    for ((name, source), labels) in names.iter().zip(sources.iter()).zip(expected_labels.iter()) {
        let lowered = mermaid_to_scene(source).unwrap_or_else(|error| panic!("{name}: mermaid lowering failed: {error}\n{source}"));
        let svg = lowered.scene.to_svg().unwrap_or_else(|error| panic!("{name}: scene must serialize to SVG: {error}"));
        for label in *labels {
            assert!(svg_contains_label(&svg, label), "{name}: SVG must draw the label `{label}`\n{svg}");
        }
        let path = out_dir.join(format!("{name}.svg"));
        fs::write(&path, &svg).expect("write diagram svg artifact");
        eprintln!("wrote {}", path.display());
    }
}

fn svg_contains_label(svg: &str, label: &str) -> bool {
    svg.contains(&format!(">{label}</text>")) || svg.contains(&format!(">{label} ")) || svg.contains(label)
}

/// DOT rides the same fence → block → lowering → HTML surfaces as Mermaid. The
/// rendered figure keeps the authored source in `data-diagram-source-hex`, so a
/// reopened document must still hold a `dot` diagram block.
#[test]
fn dot_fences_lower_to_svg_and_round_trip_html_and_markdown() {
    let source = "digraph pipeline {\n  rankdir=LR;\n  Fetch [shape=box]\n  Store [shape=box]\n  Fetch -> Store [label=\"persist\"]\n}";
    let graph = import_markdown_bytes("pipeline.md", &format!("```dot\n{source}\n```\n")).expect("import dot markdown");
    let block = single_diagram(&graph);
    assert_eq!(block, ("dot".to_string(), source.to_string()));
    assert!(export_markdown(&graph).expect("export dot markdown").contains("```dot"), "the fence must reopen as `dot`");

    let lowered = dot_to_scene(source).expect("dot lowering should succeed");
    assert!(lowered.diagnostics.is_empty(), "the subset fixture must not approximate: {:?}", lowered.diagnostics);
    let svg = lowered.scene.to_svg().expect("dot scene must serialize");
    for label in ["Fetch", "Store", "persist"] {
        assert!(svg_contains_label(&svg, label), "the DOT SVG must draw `{label}`\n{svg}");
    }

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("diagram-artifacts");
    fs::create_dir_all(&out_dir).expect("create diagram artifact dir");
    let artifact = out_dir.join("dot-pipeline.svg");
    fs::write(&artifact, &svg).expect("write dot svg artifact");
    eprintln!("wrote {}", artifact.display());

    let diagrams = HashMap::from([(diagram_id(&graph), RenderedDiagram { svg: svg.clone() })]);
    let html = export_html_with_diagrams(&graph, &diagrams).expect("export rendered dot html");
    assert!(html.contains(r#"<figure class="diagram" data-diagram-language="dot""#), "{html}");
    assert!(html.contains("data-diagram-source-hex="), "{html}");
    assert!(html.contains(&svg), "the rendered figure must inline the scene SVG: {html}");
    let reopened = import_html_bytes("pipeline.rendered.html", html.as_bytes()).expect("reopen rendered dot html");
    assert_eq!(single_diagram(&reopened), ("dot".to_string(), source.to_string()), "{html}");
}

/// A DOT construct outside the subset must fail with a byte span on the noting
/// side, and the exporter must keep the authored fence visible instead of
/// emitting half a figure.
#[test]
fn rejected_dot_keeps_the_source_fallback_visible() {
    let source = "digraph {\n  A [label=<b>bold</b>]\n}";
    let graph = import_markdown_bytes("rejected.md", &format!("```dot\n{source}\n```\n")).expect("import dot markdown");

    let error = dot_to_scene(source).expect_err("HTML labels are outside the DOT subset");
    let span = error.span().expect("the rejection must carry a span");
    assert_eq!(&source[span], "<b>bold</b>");

    let html = export_html(&graph).expect("export dot html without a rendered scene");
    assert!(html.contains(r#"<pre class="diagram" data-diagram-language="dot""#), "{html}");
    assert!(!html.contains("<figure"), "a rejected diagram must not render half a figure: {html}");
}

/// PlantUML class sources ride the same fence, block, and HTML surfaces. The
/// rendered figure keeps the authored source in `data-diagram-source-hex`, so a
/// reopened document still holds a `plantuml` diagram block.
#[test]
fn plantuml_fences_lower_to_svg_and_round_trip_html_and_markdown() {
    let source = "@startuml\nclass Order {\n  +id : String\n}\nclass Payable {\n  +pay()\n}\nOrder o-- Payable\n@enduml";
    let graph = import_markdown_bytes("orders.md", &format!("```plantuml\n{source}\n```\n")).expect("import plantuml markdown");
    let block = single_diagram(&graph);
    assert_eq!(block, ("plantuml".to_string(), source.to_string()));
    assert!(export_markdown(&graph).expect("export plantuml markdown").contains("```plantuml"), "the fence must reopen as `plantuml`");

    let lowered = plantuml_to_scene(source).expect("plantuml lowering should succeed");
    assert!(lowered.diagnostics.is_empty(), "the subset fixture must not approximate: {:?}", lowered.diagnostics);
    let svg = lowered.scene.to_svg().expect("plantuml scene must serialize");
    for label in ["Order", "+id : String", "Payable", "+pay()"] {
        assert!(svg_contains_label(&svg, label), "the PlantUML SVG must draw `{label}`\n{svg}");
    }

    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("diagram-artifacts");
    fs::create_dir_all(&out_dir).expect("create diagram artifact dir");
    let artifact = out_dir.join("plantuml-class.svg");
    fs::write(&artifact, &svg).expect("write plantuml svg artifact");
    eprintln!("wrote {} ({} bytes)", artifact.display(), svg.len());

    let diagrams = HashMap::from([(diagram_id(&graph), RenderedDiagram { svg: svg.clone() })]);
    let html = export_html_with_diagrams(&graph, &diagrams).expect("export rendered plantuml html");
    assert!(html.contains(r#"<figure class="diagram" data-diagram-language="plantuml""#), "{html}");
    assert!(html.contains("data-diagram-source-hex="), "{html}");
    assert!(html.contains(&svg), "the rendered figure must inline the scene SVG: {html}");
    let reopened = import_html_bytes("orders.rendered.html", html.as_bytes()).expect("reopen rendered plantuml html");
    assert_eq!(single_diagram(&reopened), ("plantuml".to_string(), source.to_string()), "{html}");
}

/// A PlantUML construct outside the subset must fail with the authored line span
/// and the exporter must keep the fence visible instead of emitting half a figure.
#[test]
fn rejected_plantuml_keeps_the_source_fallback_visible() {
    let source = "@startuml\nclass A\nnote right of A : detail\n@enduml";
    let graph = import_markdown_bytes("rejected.md", &format!("```plantuml\n{source}\n```\n")).expect("import plantuml markdown");

    let error = plantuml_to_scene(source).expect_err("notes are outside the PlantUML subset");
    let span = error.span().expect("the rejection must carry a span");
    assert_eq!(&source[span], "note right of A : detail");

    let html = export_html(&graph).expect("export plantuml html without a rendered scene");
    assert!(html.contains(r#"<pre class="diagram" data-diagram-language="plantuml""#), "{html}");
    assert!(!html.contains("<figure"), "a rejected diagram must not render half a figure: {html}");
}

fn diagram_id(graph: &DocumentGraph) -> NodeId {
    graph
        .blocks
        .iter()
        .find(|node| matches!(node.block, Block::Diagram { .. }))
        .map(|node| node.id)
        .expect("the document must hold one diagram")
}

fn single_diagram(graph: &DocumentGraph) -> (String, String) {
    graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Diagram { language, source } => Some((language.clone(), source.clone())),
            _ => None,
        })
        .expect("the document must hold one diagram")
}
