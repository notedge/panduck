use panduck_convert::convert_bytes;

mod common;
use common::assert_convert_report;

const BAR_SPEC: &str = r#"{"title":"Revenue","width":480,"height":320,"data":{"values":[{"region":"East","revenue":120},{"region":"West","revenue":90},{"region":"North","revenue":60}]},"mark":"bar","encoding":{"x":{"field":"region","type":"nominal"},"y":{"field":"revenue","type":"quantitative"}}}"#;

const LINE_SPEC: &str = r#"{"title":"Trend","width":480,"height":320,"data":{"values":[{"t":1,"v":10},{"t":2,"v":14},{"t":3,"v":9}]},"mark":"line","encoding":{"x":{"field":"t","type":"quantitative"},"y":{"field":"v","type":"quantitative"}}}"#;

const POINT_SPEC: &str = r#"{"title":"Scatter","width":480,"height":320,"data":{"values":[{"t":1,"v":10},{"t":2,"v":14},{"t":3,"v":9}]},"mark":"point","encoding":{"x":{"field":"t","type":"quantitative"},"y":{"field":"v","type":"quantitative"}}}"#;

fn chart_doc(language: &str, spec: &str) -> String {
    format!("# Chart\n\n```{language}\n{spec}\n```\n")
}

fn report_status(report_json: &str) -> String {
    let report: serde_json::Value = serde_json::from_str(report_json).expect("parse report");
    report["status"].as_str().expect("status string").to_string()
}

fn html_of(language: &str, spec: &str) -> panduck_convert::ConvertOutput {
    convert_bytes("markdown", "html", "chart.md", chart_doc(language, spec).into_bytes(), None).expect("convert chart markdown")
}

#[test]
fn bar_chart_renders_real_rect_geometry() {
    let output = html_of("vega-lite", BAR_SPEC);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="chart" data-chart-language="vega-lite""#), "{html}");
    assert!(html.contains(r#"<svg xmlns="http://www.w3.org/2000/svg" width="480" height="320" viewBox="0 0 480 320">"#), "{html}");
    assert!(
        html.contains(r##"<rect id="chart-bar-0" data-role="mark" data-source="East" x="67.870968" y="56" width="106.83871" height="208" fill="#4a7fbf"/>"##),
        "the first bar must carry compiled geometry: {html}"
    );
    assert!(
        html.contains(r##"<rect id="chart-bar-1" data-role="mark" data-source="West" x="186.580645" y="160" width="106.83871" height="104" fill="#4a7fbf"/>"##),
        "the second bar must carry compiled geometry: {html}"
    );
    assert!(html.contains(r#"<line id="chart-axis-x" data-role="axis" x1="56" y1="264" x2="424" y2="264""#), "{html}");
    assert!(!html.contains(r#"<pre class="chart""#), "a rendered chart must not keep the source fallback: {html}");
    assert_eq!(output.loss_count, 0, "a clean chart must stay loss-free: {}", output.report_json);
    assert_eq!(report_status(&output.report_json), "success");
    assert_convert_report(&output, "html");
}

#[test]
fn line_chart_renders_one_real_path() {
    let output = html_of("chart", LINE_SPEC);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="chart" data-chart-language="chart""#), "{html}");
    assert!(html.contains(r#"width="480" height="320" viewBox="0 0 480 320""#), "{html}");
    assert!(
        html.contains(r##"<path id="chart-line-0" data-role="mark" data-source="all" d="M56 222.4 L240 56 L424 264" fill="none" stroke="#4a7fbf" stroke-width="2"/>"##),
        "the line mark must carry the trained polyline: {html}"
    );
    assert_eq!(html.matches(r#"data-role="mark""#).count(), 1, "a line chart has one mark, not one per row: {html}");
    assert_eq!(output.loss_count, 0, "{}", output.report_json);
}

#[test]
fn point_chart_renders_one_circle_per_row() {
    let output = html_of("vega", POINT_SPEC);
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="chart" data-chart-language="vega""#), "{html}");
    for expected in [
        r##"<circle id="chart-point-0" data-role="mark" data-source="1" cx="56" cy="222.4" r="5" fill="#4a7fbf"/>"##,
        r##"<circle id="chart-point-1" data-role="mark" data-source="2" cx="240" cy="56" r="5" fill="#4a7fbf"/>"##,
        r##"<circle id="chart-point-2" data-role="mark" data-source="3" cx="424" cy="264" r="5" fill="#4a7fbf"/>"##,
    ] {
        assert!(html.contains(expected), "missing point geometry {expected}: {html}");
    }
    assert_eq!(output.loss_count, 0, "{}", output.report_json);
}

const REJECTIONS: &[(&str, &str, &str)] = &[
    (
        "davinci.chart.unsupported-transform",
        "transform",
        r#"{"data":{"values":[{"a":1}]},"transform":[{"filter":"datum.a > 0"}],"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"},"y":{"field":"a","type":"quantitative"}}}"#,
    ),
    (
        "davinci.chart.unsupported-view",
        "layer",
        r#"{"layer":[{"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"}}}]}"#,
    ),
    (
        "davinci.chart.remote-data",
        "url",
        r#"{"data":{"url":"https://example.com/data.json"},"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"}}}"#,
    ),
];

#[test]
fn unsupported_chart_constructs_keep_the_source_and_report_a_span() {
    for (code, construct, spec) in REJECTIONS {
        let output = html_of("vega-lite", spec);
        let html = output.html.as_deref().expect("html output");

        assert!(
            html.contains(r#"<pre class="chart" data-chart-language="vega-lite""#),
            "a refused chart must keep its source visible: {html}"
        );
        assert!(html.contains(construct), "the authored construct must stay readable: {html}");
        assert!(!html.contains("<figure"), "a refused chart must not emit a figure: {html}");
        assert_eq!(report_status(&output.report_json), "success_with_loss", "{code}");

        let report: serde_json::Value = serde_json::from_str(&output.report_json).expect("parse report");
        let losses = report["losses"].as_array().expect("losses array");
        assert_eq!(losses.len(), 1, "one refused fence yields one loss: {losses:?}");
        let loss = &losses[0];
        assert_eq!(loss["code"], *code, "{losses:?}");
        assert_eq!(loss["status"], "Unsupported", "{losses:?}");
        let start = loss["span"]["start"].as_u64().expect("span.start must be a real number");
        let end = loss["span"]["end"].as_u64().expect("span.end must be a real number");
        assert!(start > 0 && end > start, "the authored span must be a non-empty range: {losses:?}");
        let document = chart_doc("vega-lite", spec);
        assert!(
            document[start as usize..end as usize].contains(construct),
            "the span must cover the authored fence body: {losses:?}"
        );

        let diagnostics = report["diagnostics"]["diagnostics"].as_array().expect("diagnostics array");
        let diagnostic = diagnostics.iter().find(|entry| entry["code"] == *code).unwrap_or_else(|| {
            panic!("the unified diagnostics must keep the stable chart code {code}: {diagnostics:?}")
        });
        let args = diagnostic["message"]["args"].as_array().expect("diagnostic message args");
        let span_start = args.iter().find(|arg| arg[0] == "span.start").expect("span.start arg")[1]["u64"].as_u64();
        let span_end = args.iter().find(|arg| arg[0] == "span.end").expect("span.end arg")[1]["u64"].as_u64();
        assert_eq!(span_start, Some(start), "diagnostic span.start must match the loss span");
        assert_eq!(span_end, Some(end), "diagnostic span.end must match the loss span");
    }
}

#[test]
fn rejected_chart_loss_points_at_the_authored_block_span() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, SourceSpan};

    let source = REJECTIONS[0].2;
    let fence_start = 42;
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block_at(
        Block::Chart { language: "vega-lite".to_string(), source: source.to_string() },
        SourceSpan::new(fence_start, fence_start + source.len()),
    );

    let rendered = panduck_convert::chart::render_charts(&graph);
    assert_eq!(rendered.losses.len(), 1, "the transform must be refused: {:?}", rendered.losses);
    assert_eq!(rendered.loss_spans.len(), rendered.losses.len(), "loss spans stay aligned with losses");
    assert_eq!(
        rendered.loss_spans[0],
        SourceSpan::new(fence_start, fence_start + source.len()),
        "the chart loss carries the authored block span"
    );
    assert!(rendered.scenes.is_empty(), "a refused chart produces no scene");
}

#[test]
fn charts_and_diagrams_render_through_separate_paths() {
    let document = format!("# Mixed\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n```vega-lite\n{BAR_SPEC}\n```\n");
    let output = convert_bytes("markdown", "html", "mixed.md", document.into_bytes(), None).expect("convert mixed document");
    let html = output.html.as_deref().expect("html output");

    assert_eq!(html.matches(r#"<figure class="diagram""#).count(), 1, "{html}");
    assert_eq!(html.matches(r#"<figure class="chart""#).count(), 1, "{html}");

    let chart_at = html.find(r#"<figure class="chart""#).expect("chart figure");
    let chart_end = chart_at + html[chart_at..].find("</figure>").expect("chart figure end");
    let chart = &html[chart_at..chart_end];
    assert!(chart.contains(r#"data-chart-language="vega-lite""#), "{chart}");
    assert!(!chart.contains("data-diagram-language"), "the chart path must not reuse diagram attributes: {chart}");
    assert!(chart.contains(r#"<rect id="chart-bar-0""#), "{chart}");
    assert!(!chart.contains(r#"class="diagram""#), "chart SVG must not carry diagram classes: {chart}");

    let diagram_at = html.find(r#"<figure class="diagram""#).expect("diagram figure");
    let diagram_end = diagram_at + html[diagram_at..].find("</figure>").expect("diagram figure end");
    let diagram = &html[diagram_at..diagram_end];
    assert!(diagram.contains(r#"class="diagram""#), "{diagram}");
    assert!(!diagram.contains("chart-bar"), "the diagram path must not embed chart geometry: {diagram}");
    assert!(!diagram.contains(r#"data-role="mark""#), "diagram SVGs carry node and edge roles, not chart marks: {diagram}");
}

#[test]
fn chart_fences_survive_the_html_roundtrip_and_render_again() {
    let source = chart_doc("vega-lite", BAR_SPEC);
    let first = convert_bytes("markdown", "html", "chart.md", source.as_bytes().to_vec(), None).expect("markdown to html");
    let html = first.html.expect("html output");

    let back = convert_bytes("html", "markdown", "chart.html", html.into_bytes(), None).expect("html to markdown");
    assert!(back.markdown.contains("```vega-lite"), "the chart fence language must survive: {}", back.markdown);
    assert!(back.markdown.contains(r#""mark":"bar""#), "the authored spec must survive: {}", back.markdown);

    let again = convert_bytes("markdown", "html", "chart.md", back.markdown.into_bytes(), None).expect("re-render");
    let rerendered = again.html.expect("html output");
    assert!(
        rerendered.contains(r##"<rect id="chart-bar-0" data-role="mark" data-source="East" x="67.870968" y="56" width="106.83871" height="208" fill="#4a7fbf"/>"##),
        "the reopened fence must render the same geometry: {rerendered}"
    );
    assert_eq!(again.loss_count, 0, "{}", again.report_json);
}

#[test]
fn chart_rendering_is_byte_stable() {
    let source = chart_doc("vega-lite", BAR_SPEC);
    let first = convert_bytes("markdown", "html", "chart.md", source.as_bytes().to_vec(), None).expect("first convert");
    let second = convert_bytes("markdown", "html", "chart.md", source.as_bytes().to_vec(), None).expect("second convert");
    assert_eq!(first.html, second.html, "the same chart source must render byte-identical HTML");
}
