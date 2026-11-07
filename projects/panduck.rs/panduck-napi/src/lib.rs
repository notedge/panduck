#![deny(missing_docs)]

//! Node-API export surface for Panduck.
//!
//! This crate exposes orthogonal primitives — probe, import, transform, export
//! and project export — plus the grammar, diagram and runtime helpers. It does
//! not own CLI concerns: exit codes, usage text, argument parsing and report
//! assembly stay in the TypeScript CLI.

mod assets;
mod convert;
mod structure_guard;

pub use convert::{
    ExportResponse, ImportResponse, OfficeImportResponse, ProbeDecodeResponse, ProbeDecodedPart, ProbeIndexResponse,
    ProbeNavigationEntry, ProbeNavigationResponse, ProjectAsset, ProjectChapter, ProjectResponse, TransformResponse,
    export_document, export_markdown_project, import_document, import_office_document, probe_container_decode,
    probe_container_index, probe_navigation_toc, transform_document,
};

use std::collections::BTreeSet;

use document_convert::{READERS, WRITERS};
use document_formats::{FormatDirection, FormatStatus, capabilities};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use oak_core::{Lexer, TokenType, lexer::NoLexerCache, source::SourceText};

/// Format wired here through `document-formats` because `document-convert`
/// keeps no aggregate reader or writer entry for it.
///
/// Pandoc JSON is a format-layer exchange codec, so the aggregate route table
/// cannot advertise it. The binding wires it directly and derives availability
/// from the same capability table, keeping the advertised route and the served
/// route in one place.
const PANDOC_JSON: &str = "pandoc-json";

fn reader_wired(format: &str) -> bool {
    READERS.contains(&format) || format == PANDOC_JSON
}

fn writer_wired(format: &str) -> bool {
    WRITERS.contains(&format) || format == PANDOC_JSON
}

fn capability_routable(format: &str, direction: FormatDirection) -> bool {
    capabilities()
        .into_iter()
        .find(|capability| capability.id == format && capability.direction == direction)
        .is_some_and(|capability| matches!(capability.status, FormatStatus::Ready | FormatStatus::Partial))
}

/// Whether this binding can read `format` and report its gaps per document.
fn reader_routable(format: &str) -> bool {
    reader_wired(format)
        && capability_routable(format, FormatDirection::Import)
        && crate::convert::serves_read(format)
}

/// Whether this binding can write `format`.
fn writer_routable(format: &str) -> bool {
    writer_wired(format)
        && capability_routable(format, FormatDirection::Export)
        && crate::convert::serves_write(format)
}

fn route_supported(from: &str, to: &str) -> bool {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    reader_routable(&from) && writer_routable(&to)
}

/// Panduck-owned render code for a language with no diagram frontend.
///
/// It stays outside the DaVinci vocabulary, which lives in
/// `plotter_diagram::codes` and is never re-spelled in this crate.
const UNSUPPORTED_LANGUAGE: &str = "panduck.render.unsupported_language";

/// One lexical token produced by a Panduck source grammar.
#[napi(object)]
pub struct LexToken {
    /// Grammar token kind, such as `Heading1`, `TagName` or `Eof`.
    pub kind: String,
    /// Source slice covered by this token.
    pub text: String,
    /// Byte offset of the first source character.
    pub start: u32,
    /// Byte offset one past the last source character.
    pub end: u32,
    /// Whether the grammar classifies this token as trivia.
    pub ignored: bool,
}

/// N-API lexical analysis response.
#[napi(object)]
pub struct LexResponse {
    /// False when the grammar reported a fatal lexical error.
    pub success: bool,
    /// Tokens in source order, including grammar trivia.
    pub tokens: Vec<LexToken>,
    /// Lexical diagnostics rendered as text.
    pub diagnostics: Vec<String>,
}

/// Returns the Panduck N-API binding version.
#[napi]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists formats wired into this build (reader ∪ writer).
#[napi]
pub fn supported_formats() -> Vec<String> {
    let mut set = BTreeSet::new();
    for f in READERS {
        set.insert(*f);
    }
    for f in WRITERS {
        set.insert(*f);
    }
    if reader_routable(PANDOC_JSON) || writer_routable(PANDOC_JSON) {
        set.insert(PANDOC_JSON);
    }
    set.into_iter().map(|s| s.to_string()).collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[napi]
pub fn is_supported_format(name: String) -> bool {
    supported_formats().iter().any(|f| f.eq_ignore_ascii_case(name.as_str()))
}

/// Returns whether a conversion route is wired in this build.
#[napi]
pub fn supports_conversion(from: String, to: String) -> bool {
    route_supported(&from, &to)
}

/// Lexically analyses source text with the grammar Panduck reads it as.
///
/// Supported `language` values are `notedown`, `markdown`, `html` and `xml`.
/// `xml` covers the document parts inside OPC containers. The token stream is
/// the same one the matching reader feeds into its parser, so consumers can
/// build editors, linters or incremental caches on top of it.
#[napi]
pub fn lex_document(language: String, source: String) -> LexResponse {
    match language.to_ascii_lowercase().as_str() {
        "notedown" | "nd" => {
            let config = oak_notedown::NoteLanguage::default();
            lex_tokens::<oak_notedown::NoteLanguage, _>(&source, &oak_notedown::NoteLexer::new(&config))
        }
        "markdown" | "md" => {
            let config = markdown_grammar_language();
            lex_tokens::<oak_markdown::MarkdownLanguage, _>(&source, &oak_markdown::MarkdownLexer::new(&config))
        }
        "html" => {
            let config = oak_html::HtmlLanguage::default();
            lex_tokens::<oak_html::HtmlLanguage, _>(&source, &oak_html::HtmlLexer::new(&config))
        }
        "xml" => {
            let config = oak_xml::XmlLanguage::new();
            lex_tokens::<oak_xml::XmlLanguage, _>(&source, &oak_xml::XmlLexer::new(&config))
        }
        other => LexResponse {
            success: false,
            tokens: Vec::new(),
            diagnostics: vec![format!("lex.unsupported_language: `{other}`")],
        },
    }
}

/// Markdown grammar exposed by the lexer: GFM plus math and footnotes.
///
/// The reader layer applies a narrower parse policy, but lexical analysis is a
/// grammar level contract: a token stream that silently cannot represent
/// CommonMark core constructs (indented code blocks, autolinks, setext
/// headings) would be unusable for editors and linters. What the reader does
/// with those tokens stays a reader concern.
fn markdown_grammar_language() -> oak_markdown::MarkdownLanguage {
    let mut language = oak_markdown::MarkdownLanguage::github_flavored();
    language.allow_math = true;
    language.allow_footnotes = true;
    language
}

fn lex_tokens<L, X>(source: &str, lexer: &X) -> LexResponse
where
    L: oak_core::Language,
    X: Lexer<L>,
{
    let text = SourceText::new(source.to_string());
    let mut cache = NoLexerCache;
    let output = lexer.lex(&text, &[], &mut cache);
    let diagnostics = output.diagnostics.iter().map(|error| error.to_string()).collect::<Vec<_>>();
    match output.result {
        Ok(tokens) => LexResponse {
            success: true,
            tokens: tokens
                .iter()
                .map(|token| LexToken {
                    kind: format!("{:?}", token.kind),
                    text: source[token.span.start..token.span.end].to_string(),
                    start: u32::try_from(token.span.start).unwrap_or(u32::MAX),
                    end: u32::try_from(token.span.end).unwrap_or(u32::MAX),
                    ignored: token.kind.is_ignored(),
                })
                .collect(),
            diagnostics,
        },
        Err(error) => {
            let mut all = diagnostics;
            all.push(error.to_string());
            LexResponse { success: false, tokens: Vec::new(), diagnostics: all }
        }
    }
}

/// One diagram diagnostic, fatal or non-fatal.
#[napi(object)]
pub struct DiagramDiagnostic {
    /// Stable diagnostic code such as `davinci.diagram.unsupported`.
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// Human readable message.
    pub message: String,
    /// Source slice the diagnostic points at, empty when it has no span.
    pub span_text: String,
}

/// N-API diagram render response.
#[napi(object)]
pub struct DiagramRenderResponse {
    /// False when the source was rejected or rendering failed.
    pub success: bool,
    /// SVG document for accepted sources.
    pub svg: Option<String>,
    /// Diagnostics in source order.
    pub diagnostics: Vec<DiagramDiagnostic>,
}

/// Renders a diagram source through the matching plotter frontend.
///
/// Supported `language` values are `dot`, `mermaid`, `plantuml` and `tikz`. The
/// result is deterministic SVG with caller scoped ids, which lets consumers
/// embed the same diagram several times in one page. TikZ lowers to an authored
/// scene, so its paint is serialized verbatim instead of being recolored by the
/// diagram theme.
#[napi]
pub fn render_diagram(language: String, source: String) -> DiagramRenderResponse {
    match language.to_ascii_lowercase().as_str() {
        "dot" => lower_diagram(plotter_input_dot::dot_to_scene(&source), &source),
        "mermaid" => lower_diagram(plotter_input_mermaid::mermaid_to_scene(&source), &source),
        "plantuml" => lower_diagram(plotter_input_plantuml::plantuml_to_scene(&source), &source),
        "tikz" => match plotter_input_tikz::tikz_to_scene(&source) {
            Ok(scene) => render_authored_scene(scene, Vec::new()),
            Err(error) => diagram_error_response(error, &source),
        },
        other => DiagramRenderResponse {
            success: false,
            svg: None,
            diagnostics: vec![DiagramDiagnostic {
                code: UNSUPPORTED_LANGUAGE.to_string(),
                severity: "error".to_string(),
                message: format!("`{other}` has no diagram frontend in Panduck"),
                span_text: String::new(),
            }],
        },
    }
}

fn lower_diagram(
    lowered: std::result::Result<plotter_diagram::LoweredDiagram, plotter_diagram::DiagramError>,
    source: &str,
) -> DiagramRenderResponse {
    let lowered = match lowered {
        Ok(lowered) => lowered,
        Err(error) => return diagram_error_response(error, source),
    };
    let diagnostics: Vec<DiagramDiagnostic> = lowered.diagnostics.iter().map(diagram_diagnostic).collect();
    render_scene(lowered.scene, diagnostics)
}

fn diagram_error_response(error: plotter_diagram::DiagramError, source: &str) -> DiagramRenderResponse {
    let span_text = error
        .span()
        .and_then(|span| source.get(span))
        .unwrap_or_default()
        .to_string();
    DiagramRenderResponse {
        success: false,
        svg: None,
        diagnostics: vec![DiagramDiagnostic {
            code: error.code().to_string(),
            severity: "error".to_string(),
            message: error.message().to_string(),
            span_text,
        }],
    }
}

fn render_scene(
    scene: plotter_scene::DisplayScene,
    mut diagnostics: Vec<DiagramDiagnostic>,
) -> DiagramRenderResponse {
    match scene.to_svg_with_hooks("panduck-diagram", "panduck-diagram", &plotter_types::DIAGRAM_THEME) {
        Ok(svg) => DiagramRenderResponse { success: true, svg: Some(svg), diagnostics },
        Err(error) => {
            diagnostics.push(DiagramDiagnostic {
                code: plotter_diagram::codes::SERIALIZATION_FAILED.to_string(),
                severity: "error".to_string(),
                message: error.to_string(),
                span_text: String::new(),
            });
            DiagramRenderResponse { success: false, svg: None, diagnostics }
        }
    }
}

/// Serializes an authored scene with its own paint.
///
/// TikZ shapes carry the colour, fill and opacity written in the source, not a
/// semantic role, so the diagram palette cannot paint them. Serializing without
/// page hooks keeps those attributes instead of dropping them to an unpainted
/// path that renders nothing.
fn render_authored_scene(
    scene: plotter_scene::DisplayScene,
    mut diagnostics: Vec<DiagramDiagnostic>,
) -> DiagramRenderResponse {
    match scene.to_svg() {
        Ok(svg) => DiagramRenderResponse { success: true, svg: Some(svg), diagnostics },
        Err(error) => {
            diagnostics.push(DiagramDiagnostic {
                code: plotter_diagram::codes::SERIALIZATION_FAILED.to_string(),
                severity: "error".to_string(),
                message: error.to_string(),
                span_text: String::new(),
            });
            DiagramRenderResponse { success: false, svg: None, diagnostics }
        }
    }
}

/// One diagnostic produced while lowering or rendering a chart.
#[napi(object)]
pub struct ChartDiagnostic {
    /// Stable diagnostic code such as `davinci.chart.unsupported-mark`.
    pub code: String,
    /// Severity label such as `error` or `warning`.
    pub severity: String,
    /// Human readable explanation.
    pub message: String,
    /// Source text covered by the diagnostic, empty when it has no span.
    pub span_text: String,
    /// JSON path inside the Vega-Lite spec, empty when the diagnostic has none.
    pub path: String,
}

/// N-API chart render response.
#[napi(object)]
pub struct ChartRenderResponse {
    /// True when the spec lowered and rendered without an error diagnostic.
    pub success: bool,
    /// Rendered SVG document, `None` on failure.
    pub svg: Option<String>,
    /// Diagnostics emitted while lowering and rendering.
    pub diagnostics: Vec<ChartDiagnostic>,
}

/// Renders a Vega-Lite spec to an SVG scene through `plotter.chart`.
///
/// The input is the canonical Vega-Lite JSON payload, so callers do not need a
/// separate compilation step. A spec that cannot be lowered is reported through
/// the same diagnostics shape as warnings, which keeps failure and degradation
/// in one channel.
#[napi]
pub fn render_chart(vega_lite_json: String) -> ChartRenderResponse {
    let chart = match plotter_chart::vega_lite::lower(&vega_lite_json) {
        Ok(chart) => chart,
        Err(error) => {
            return ChartRenderResponse {
                success: false,
                svg: None,
                diagnostics: vec![ChartDiagnostic {
                    code: error.code().to_string(),
                    severity: "error".to_string(),
                    message: error.message().to_string(),
                    span_text: error
                        .span()
                        .and_then(|span| vega_lite_json.get(span))
                        .unwrap_or_default()
                        .to_string(),
                    path: error.path().unwrap_or_default().to_string(),
                }],
            };
        }
    };
    let mut diagnostics: Vec<ChartDiagnostic> = chart
        .diagnostics
        .iter()
        .map(|diagnostic| ChartDiagnostic {
            code: diagnostic.code().as_str().to_string(),
            severity: diagnostic.severity().to_string(),
            message: diagnostic
                .message()
                .fallback()
                .unwrap_or_else(|| diagnostic.message().key())
                .to_string(),
            span_text: String::new(),
            path: String::new(),
        })
        .collect();
    match chart.to_svg_with_hooks("panduck-chart", "panduck-chart") {
        Ok(svg) => ChartRenderResponse { success: true, svg: Some(svg), diagnostics },
        Err(error) => {
            diagnostics.push(ChartDiagnostic {
                code: error.code().to_string(),
                severity: "error".to_string(),
                message: error.to_string(),
                span_text: String::new(),
                path: String::new(),
            });
            ChartRenderResponse { success: false, svg: None, diagnostics }
        }
    }
}

/// Ordered incremental passes implied by one runtime action or scope set.
#[napi(object)]
pub struct RuntimePassResponse {
    /// Passes in execution order, from upstream data to downstream paint.
    pub passes: Vec<String>,
    /// Invalidation scopes that produced the passes.
    pub scopes: Vec<String>,
    /// Set when the requested action is outside the runtime vocabulary.
    pub error: Option<String>,
}

/// Derives the incremental passes one semantic action invalidates.
///
/// Hosts call this before running a frame so a hover stays a pure overlay
/// repaint and never triggers data reads, measurement or layout. Unknown
/// actions are an error instead of silently falling back to a full recompile.
#[napi]
pub fn runtime_passes_for_action(action: String, target: Option<String>) -> RuntimePassResponse {
    let mapped = match runtime_action(&action, target.as_deref()) {
        Some(mapped) => mapped,
        None => {
            return RuntimePassResponse {
                passes: Vec::new(),
                scopes: Vec::new(),
                error: Some(format!("`{action}` is not a plotter runtime action")),
            };
        }
    };
    let invalidation = plotter_runtime::invalidate(&mapped);
    runtime_pass_response(&invalidation.scopes)
}

/// Derives the incremental passes one or more invalidation scopes imply.
#[napi]
pub fn runtime_passes_for_scopes(scopes: Vec<String>) -> RuntimePassResponse {
    let mut set = plotter_runtime::InvalidationSet::empty();
    for name in &scopes {
        match invalidation_scope(name) {
            Some(scope) => {
                set.insert(scope);
            }
            None => {
                return RuntimePassResponse {
                    passes: Vec::new(),
                    scopes: Vec::new(),
                    error: Some(format!("`{name}` is not a plotter invalidation scope")),
                };
            }
        }
    }
    runtime_pass_response(&set)
}

fn runtime_pass_response(invalidation: &plotter_runtime::InvalidationSet) -> RuntimePassResponse {
    let closed = invalidation.closure();
    RuntimePassResponse {
        passes: plotter_runtime::derive_steps(&closed)
            .into_iter()
            .map(|pass| format!("{pass:?}"))
            .collect(),
        scopes: closed.iter().map(|scope| format!("{scope:?}")).collect(),
        error: None,
    }
}

fn runtime_action(name: &str, target: Option<&str>) -> Option<plotter_runtime::Action> {
    use plotter_runtime::Action;

    let id = || target.map(str::to_string);
    Some(match name {
        "Hover" => Action::Hover(id()),
        "Focus" => Action::Focus(id()),
        "Press" => Action::Press(id()),
        "Release" => Action::Release,
        "Select" => Action::Select {
            target: target.unwrap_or_default().to_string(),
            additive: false,
        },
        "ClearSelection" => Action::ClearSelection,
        "Zoom" => Action::Zoom { factor: 1.0, anchor: None },
        "Pan" => Action::Pan { dx: 0.0, dy: 0.0 },
        "Resize" => Action::Resize { width: 0.0, height: 0.0 },
        "SetParameter" => Action::SetParameter {
            name: target.unwrap_or_default().to_string(),
            value: 0.0,
        },
        "SetTheme" => Action::SetTheme(id()),
        "Play" => Action::Play,
        "Pause" => Action::Pause,
        "Seek" => Action::Seek(0.0),
        "Advance" => Action::Advance(0.0),
        _ => return None,
    })
}

fn invalidation_scope(name: &str) -> Option<plotter_runtime::InvalidationScope> {
    use plotter_runtime::InvalidationScope;

    Some(match name {
        "Overlay" => InvalidationScope::Overlay,
        "Style" => InvalidationScope::Style,
        "Viewport" => InvalidationScope::Viewport,
        "Measurement" => InvalidationScope::Measurement,
        "Layout" => InvalidationScope::Layout,
        "Data" => InvalidationScope::Data,
        "Motion" => InvalidationScope::Motion,
        _ => return None,
    })
}

fn diagram_diagnostic(diagnostic: &boost_diagnostic::Diagnostic) -> DiagramDiagnostic {
    DiagramDiagnostic {
        code: diagnostic.code().as_str().to_string(),
        severity: diagnostic.severity().to_string(),
        message: diagnostic
            .message()
            .fallback()
            .unwrap_or_else(|| diagnostic.message().key())
            .to_string(),
        span_text: String::new(),
    }
}

/// One structural invariant violation found on a document graph.
#[napi(object)]
pub struct IrValidationIssue {
    /// Invariant name such as `UnknownInlineLinkTarget` or `ContainmentCycle`.
    pub code: String,
    /// Graph location the issue points at, such as `node:7` or `relation:3`.
    pub path: String,
    /// Human readable explanation of the violation.
    pub detail: String,
}

/// N-API IR validation response.
#[napi(object)]
pub struct IrValidationResponse {
    /// True when the graph satisfies every structural invariant.
    pub valid: bool,
    /// Violations in the order `DocumentGraph::validate` discovered them.
    pub issues: Vec<IrValidationIssue>,
    /// Set when the payload was not a deserializable document graph.
    pub error: Option<String>,
}

/// Validates a serialized `document.ir` graph against its structural invariants.
///
/// The graph JSON uses the same schema as `document.ir`'s serde encoding, so
/// consumers that build semantic graphs outside Rust can check identity
/// uniqueness, endpoint existence, reference integrity and containment
/// acyclicity before trusting the payload. A payload that is not a document
/// graph is reported through `error` instead of being mixed into `issues`.
#[napi]
pub fn validate_document_graph(graph_json: String) -> IrValidationResponse {
    let graph: document_ir::DocumentGraph = match serde_json::from_str(&graph_json) {
        Ok(graph) => graph,
        Err(error) => {
            return IrValidationResponse {
                valid: false,
                issues: Vec::new(),
                error: Some(format!("graph is not a document graph: {error}")),
            };
        }
    };
    let report = graph.validate();
    IrValidationResponse {
        valid: report.is_valid(),
        issues: report.issues.iter().map(issue_record).collect(),
        error: None,
    }
}

fn issue_record(issue: &document_ir::ValidationIssue) -> IrValidationIssue {
    use document_ir::ValidationIssue as Issue;

    let (code, path, detail) = match issue {
        Issue::DuplicateNodeId { id } => (
            "DuplicateNodeId",
            format!("node:{}", id.0),
            format!("node id {} is pushed more than once", id.0),
        ),
        Issue::DuplicateAssetId { id } => (
            "DuplicateAssetId",
            format!("asset:{}", id.0),
            format!("asset id {} is pushed more than once", id.0),
        ),
        Issue::DuplicateLinkId { id } => (
            "DuplicateLinkId",
            format!("relation:{}", id.0),
            format!("relation id {} is pushed more than once", id.0),
        ),
        Issue::UnknownNode { id } => (
            "UnknownNode",
            format!("node:{}", id.0),
            format!("provenance points at node {} which is absent from `blocks`", id.0),
        ),
        Issue::UnknownSectionChild { section, child } => (
            "UnknownSectionChild",
            format!("node:{}", section.0),
            format!("section {} names child {} which is absent from `blocks`", section.0, child.0),
        ),
        Issue::UnknownListChild { list, child } => (
            "UnknownListChild",
            format!("node:{}", list.0),
            format!("list {} names child {} which is absent from `blocks`", list.0, child.0),
        ),
        Issue::UnknownQuoteChild { quote, child } => (
            "UnknownQuoteChild",
            format!("node:{}", quote.0),
            format!("quote {} names child {} which is absent from `blocks`", quote.0, child.0),
        ),
        Issue::UnknownTableCellChild { table, row_index, cell_index, child } => (
            "UnknownTableCellChild",
            format!("node:{}[{}][{}]", table.0, row_index, cell_index),
            format!(
                "table {} cell ({}, {}) names child {} which is absent from `blocks`",
                table.0, row_index, cell_index, child.0
            ),
        ),
        Issue::UnknownInlineReference { from, target } => (
            "UnknownInlineReference",
            format!("node:{}", from.0),
            format!("reference on node {} names node {} which is absent", from.0, target.0),
        ),
        Issue::UnknownInlineLinkTarget { from, target } => (
            "UnknownInlineLinkTarget",
            format!("node:{}", from.0),
            format!("link on node {} targets {target:?} which was never pushed", from.0),
        ),
        Issue::UnknownInlineImageAsset { from, asset } => (
            "UnknownInlineImageAsset",
            format!("node:{}", from.0),
            format!("image on node {} binds asset {} which was never pushed", from.0, asset.0),
        ),
        Issue::EmptyInlineLabel { from, kind } => (
            "EmptyInlineLabel",
            format!("node:{}", from.0),
            format!("inline {kind} on node {} carries an empty label", from.0),
        ),
        Issue::EmptyInlineImageSource { from } => (
            "EmptyInlineImageSource",
            format!("node:{}", from.0),
            format!(
                "image on node {} has neither a source location nor a bound asset",
                from.0
            ),
        ),
        Issue::EmptyInlineAttributes { from } => (
            "EmptyInlineAttributes",
            format!("node:{}", from.0),
            format!("attributed inline on node {} carries no attribute", from.0),
        ),
        Issue::EmptyInlineAttributeName { from, slot } => (
            "EmptyInlineAttributeName",
            format!("node:{}", from.0),
            format!("inline attribute {slot} on node {} is empty", from.0),
        ),
        Issue::EmptyInlineStyle { from } => (
            "EmptyInlineStyle",
            format!("node:{}", from.0),
            format!("styled inline on node {} carries an empty style name", from.0),
        ),
        Issue::UnknownRelationEndpoint { link } => (
            "UnknownRelationEndpoint",
            format!("relation:{}", link.0),
            format!("relation {} has an endpoint that is neither a node, asset nor document", link.0),
        ),
        Issue::ContainmentCycle { from, to } => (
            "ContainmentCycle",
            format!("node:{}", from.0),
            format!("containment from node {} loops back to node {}", from.0, to.0),
        ),
        Issue::MissingListItemNumber { list, item_index } => (
            "MissingListItemNumber",
            format!("node:{}[{}]", list.0, item_index),
            format!("ordered list {} leaves item {} without a number", list.0, item_index),
        ),
        Issue::OutOfOrderListItemNumber { list, item_index, found, previous } => (
            "OutOfOrderListItemNumber",
            format!("node:{}[{}]", list.0, item_index),
            format!(
                "ordered list {} item {} has number {:?} after {:?}",
                list.0, item_index, found, previous
            ),
        ),
        Issue::UnexpectedListItemNumber { list, item_index, found } => (
            "UnexpectedListItemNumber",
            format!("node:{}[{}]", list.0, item_index),
            format!("unordered list {} item {} carries number {}", list.0, item_index, found),
        ),
        Issue::ZeroColspan { table, row_index, cell_index } => (
            "ZeroColspan",
            format!("node:{}[{}][{}]", table.0, row_index, cell_index),
            format!("table {} cell ({}, {}) declares colspan 0", table.0, row_index, cell_index),
        ),
        Issue::ZeroRowspan { table, row_index, cell_index } => (
            "ZeroRowspan",
            format!("node:{}[{}][{}]", table.0, row_index, cell_index),
            format!("table {} cell ({}, {}) declares rowspan 0", table.0, row_index, cell_index),
        ),
        Issue::ColspanOutOfBounds { table, row_index, cell_index, found, max_columns } => (
            "ColspanOutOfBounds",
            format!("node:{}[{}][{}]", table.0, row_index, cell_index),
            format!(
                "table {} cell ({}, {}) declares colspan {} with only {} columns",
                table.0, row_index, cell_index, found, max_columns
            ),
        ),
        Issue::RowspanOutOfBounds { table, row_index, cell_index, found, total_rows } => (
            "RowspanOutOfBounds",
            format!("node:{}[{}][{}]", table.0, row_index, cell_index),
            format!(
                "table {} cell ({}, {}) declares rowspan {} with only {} rows",
                table.0, row_index, cell_index, found, total_rows
            ),
        ),
        Issue::EmptyMetadataKey { path } => (
            "EmptyMetadataKey",
            format!("metadata:{path}"),
            format!("metadata entry at `{path}` has an empty key"),
        ),
        Issue::DuplicateMetadataKey { key, first, repeated } => (
            "DuplicateMetadataKey",
            format!("metadata:{key}"),
            format!("metadata key `{key}` repeats at index {repeated}, first seen at index {first}"),
        ),
        Issue::ReservedMetadataKey { key } => (
            "ReservedMetadataKey",
            format!("metadata:{key}"),
            format!("metadata key `{key}` is owned by a fixed document metadata field"),
        ),
        Issue::NonFiniteMetadataFloat { path } => (
            "NonFiniteMetadataFloat",
            format!("metadata:{path}"),
            format!("metadata value at `{path}` is a non-finite float"),
        ),
    };
    IrValidationIssue {
        code: code.to_string(),
        path,
        detail,
    }
}

/// Install a JSON lines file sink on the global logger facade.
#[napi]
pub fn install_logger_log_file(path: String) -> Result<()> {
    boost_log::install_global_file_sink(&path).map_err(|error| Error::from_reason(error.to_string()))
}

/// Lists wired conversion routes as `from:to` strings.
#[napi]
pub fn supported_conversions() -> Vec<String> {
    let readers = READERS.iter().copied().chain(std::iter::once(PANDOC_JSON));
    let writers = WRITERS.iter().copied().chain(std::iter::once(PANDOC_JSON));
    let mut routes = Vec::new();
    for from in readers {
        for to in writers.clone() {
            if route_supported(from, to) {
                routes.push(format!("{from}:{to}"));
            }
        }
    }
    routes
}

/// Returns whether a source format can be converted into a Markdown project.
#[napi]
pub fn supports_markdown_project(from: String) -> bool {
    reader_routable(&from.to_ascii_lowercase())
}

/// One text run the layout pass reserved space for before PDF painting.
#[napi(object)]
pub struct PdfLayoutRun {
    /// Zero based page index the run was placed on.
    pub page: u32,
    /// Left edge in points, measured from the page left.
    pub x: f64,
    /// Baseline in points, measured from the page top.
    pub y: f64,
    /// Reserved horizontal width in points, measured by the same advances the
    /// line breaker used.
    pub width: f64,
    /// Font size in points.
    pub size: f64,
    /// Line text carried by the run.
    pub text: String,
    /// Resolved font family name.
    pub font_family: String,
    /// Debug name of the resolved face style, such as `Regular` or `Italic`.
    pub style: String,
}

/// Page box produced by the layout pass.
#[napi(object)]
pub struct PdfLayoutPage {
    /// Page width in points.
    pub width: f64,
    /// Page height in points.
    pub height: f64,
}

/// Layout runs plus the PDF bytes they were painted into.
#[napi(object)]
pub struct PdfLayoutProbeResponse {
    /// True when the probe produced a PDF.
    pub success: bool,
    /// PDF bytes rendered from the same graph by the production writer.
    pub pdf: Option<Buffer>,
    /// Text runs in paint order, one entry per `document-layout` `TextRun`.
    pub runs: Vec<PdfLayoutRun>,
    /// Page boxes in document order.
    pub pages: Vec<PdfLayoutPage>,
    /// Layout diagnostics rendered as text.
    pub diagnostics: Vec<String>,
    /// Set when reading, laying out or rendering failed.
    pub error: Option<String>,
}

/// Lays out Markdown text and renders its PDF in one call.
///
/// The response pairs the PDF bytes with the `TextRun` widths from
/// `document_layout::document::layout_with_diagnostics` — the exact call the
/// production PDF writer makes internally. Consumers can therefore check that
/// a run's painted advance matches the width the layout reserved, instead of
/// hard coding absolute point values that move whenever font metrics change.
/// This is a read-only probe: it does not add a route and does not change the
/// production conversion pipeline.
#[napi]
pub fn pdf_layout_probe(label: String, bytes: Buffer) -> PdfLayoutProbeResponse {
    crate::assets::install_math_fonts();
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text.to_string(),
        Err(error) => return pdf_probe_error(format!("input `{label}` is not UTF-8 Markdown: {error}")),
    };
    let graph = match document_convert::read_markdown_bytes(&label, text) {
        Ok(graph) => graph,
        Err(error) => return pdf_probe_error(error.to_string()),
    };
    let (plan, layout_diagnostics) = document_layout::document::layout_with_diagnostics(
        &graph,
        &document_layout::LayoutHint::a4(),
        &document_layout::Diagrams::default(),
        crate::assets::body_metrics(),
    );
    let pdf = match document_convert::write_document_pdf(&graph, &crate::assets::body_set()) {
        Ok(pdf) => pdf,
        Err(error) => return pdf_probe_error(error.to_string()),
    };
    let mut runs = Vec::new();
    let mut pages = Vec::new();
    for (index, page) in plan.pages.iter().enumerate() {
        pages.push(PdfLayoutPage { width: page.width, height: page.height });
        for item in &page.items {
            if let document_layout::plan::LayoutItem::TextRun(run) = item {
                runs.push(PdfLayoutRun {
                    page: u32::try_from(index).unwrap_or(u32::MAX),
                    x: run.x,
                    y: run.y,
                    width: run.width,
                    size: run.size,
                    text: run.text.clone(),
                    font_family: run.font_family.clone(),
                    style: format!("{:?}", run.style),
                });
            }
        }
    }
    PdfLayoutProbeResponse {
        success: true,
        pdf: Some(Buffer::from(pdf)),
        runs,
        pages,
        diagnostics: layout_diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect(),
        error: None,
    }
}

fn pdf_probe_error(message: String) -> PdfLayoutProbeResponse {
    PdfLayoutProbeResponse {
        success: false,
        pdf: None,
        runs: Vec::new(),
        pages: Vec::new(),
        diagnostics: Vec::new(),
        error: Some(message),
    }
}

/// One issue reported by an office reader or its canvas projection.
#[napi(object)]
pub struct OfficeIssue {
    /// Stable diagnostic code such as `import.xlsx.chart`.
    pub code: String,
    /// `partial` or `unsupported`.
    pub status: String,
    /// Human readable explanation.
    pub message: String,
    /// Sheet name when the issue is workbook scoped.
    pub sheet: Option<String>,
    /// A1 reference when the issue is cell scoped.
    pub cell: Option<String>,
    /// One based slide number when the issue is slide scoped.
    pub slide: Option<u32>,
    /// Shape id when the issue is shape scoped.
    pub shape: Option<u32>,
}

impl OfficeIssue {
    fn xlsx(issue: &document_formats::import::xlsx::XlsxIssue) -> Self {
        Self {
            code: issue.code.clone(),
            status: format!("{:?}", issue.status).to_lowercase(),
            message: issue.message.clone(),
            sheet: issue.sheet.clone(),
            cell: issue.cell.clone(),
            slide: None,
            shape: None,
        }
    }

    fn pptx(issue: &document_formats::import::pptx::PptxIssue) -> Self {
        Self {
            code: issue.code.clone(),
            status: format!("{:?}", issue.status).to_lowercase(),
            message: issue.message.clone(),
            sheet: None,
            cell: None,
            slide: issue.slide,
            shape: issue.shape,
        }
    }
}

/// One paginated page of a sheet grid view.
#[napi(object)]
pub struct SheetPageInfo {
    /// Sheet name.
    pub sheet: String,
    /// One based page number inside the sheet.
    pub number: u32,
    /// Page count of the sheet.
    pub total: u32,
    /// First painted row.
    pub first_row: u32,
    /// Last painted row.
    pub last_row: u32,
    /// First painted column.
    pub first_column: u32,
    /// Last painted column.
    pub last_column: u32,
    /// Cells drawn on this page.
    pub cell_count: u32,
    /// DOM id used by screenshot tooling.
    pub id: String,
}

/// Workbook grid view plus the counts an acceptance report quotes.
#[napi(object)]
pub struct SpreadsheetRenderResponse {
    /// Self contained paginated HTML document.
    pub html: String,
    /// Sheet count in the workbook.
    pub sheet_count: u32,
    /// Visible sheet count.
    pub visible_sheet_count: u32,
    /// Cells carrying a value.
    pub cell_count: u32,
    /// Non whitespace characters across display text.
    pub non_whitespace_chars: u32,
    /// Pages emitted across every visible sheet.
    pub page_count: u32,
    /// Page metadata in document order.
    pub pages: Vec<SheetPageInfo>,
    /// Import and view issues.
    pub issues: Vec<OfficeIssue>,
}

/// Renders an XLSX package into a paginated sheet grid view.
///
/// The bytes are read into the format native workbook model first, so the
/// projection keeps number formats, frozen panes and clamps instead of
/// lowering through the semantic IR.
#[napi]
pub fn render_spreadsheet(label: String, bytes: Buffer) -> Result<SpreadsheetRenderResponse> {
    let workbook = document_formats::import::xlsx::import_xlsx_bytes(&label, &bytes)
        .map_err(|error| Error::from_reason(error.to_string()))?;
    let render = document_formats::import::xlsx::render_workbook_html(
        &workbook,
        &document_formats::import::xlsx::RenderOptions::default(),
    );
    // The view render starts from the workbook issues, so reading them again here
    // would report every import code twice.
    let issues: Vec<OfficeIssue> = render.issues.iter().map(OfficeIssue::xlsx).collect();
    Ok(SpreadsheetRenderResponse {
        html: render.html,
        sheet_count: workbook.stats.sheet_count as u32,
        visible_sheet_count: workbook.stats.visible_sheet_count as u32,
        cell_count: workbook.stats.cell_count as u32,
        non_whitespace_chars: workbook.stats.non_whitespace_chars as u32,
        page_count: render.pages.len() as u32,
        pages: render
            .pages
            .iter()
            .map(|page| SheetPageInfo {
                sheet: page.sheet.clone(),
                number: page.number as u32,
                total: page.total as u32,
                first_row: page.first_row,
                last_row: page.last_row,
                first_column: page.first_column,
                last_column: page.last_column,
                cell_count: page.cell_count as u32,
                id: page.id.clone(),
            })
            .collect(),
        issues,
    })
}

/// One painted slide canvas.
#[napi(object)]
pub struct SlideCanvas {
    /// One based slide number.
    pub index: u32,
    /// Canvas width in CSS pixels.
    pub width_px: f64,
    /// Canvas height in CSS pixels.
    pub height_px: f64,
    /// Slide SVG.
    pub svg: String,
    /// Object level deviations recorded for this slide.
    pub deviation_count: u32,
}

/// A presentation chart lowered into a plotter scene and painted as SVG.
#[napi(object)]
pub struct ChartSceneInfo {
    /// One based slide number.
    pub slide: u32,
    /// Shape id carrying the chart frame.
    pub shape: u32,
    /// Chart kind declared by the package, such as `barChart`.
    pub kind: Option<String>,
    /// Semantic scene node count.
    pub node_count: u32,
    /// Scene width in CSS pixels.
    pub width_px: f64,
    /// Scene height in CSS pixels.
    pub height_px: f64,
    /// Chart SVG placed at the chart frame.
    pub svg: String,
    /// Set when the chart family has no scene lowering.
    pub error: Option<String>,
}

/// Slide canvases, chart scenes and the counts an acceptance report quotes.
#[napi(object)]
pub struct PresentationRenderResponse {
    /// Self contained HTML document with one section per slide.
    pub html: String,
    /// Slides in the presentation.
    pub slide_count: u32,
    /// Shapes across every slide.
    pub shape_count: u32,
    /// Pictures embedded in slides.
    pub image_count: u32,
    /// Tables on slides.
    pub table_count: u32,
    /// Charts on slides.
    pub chart_count: u32,
    /// Notes pages.
    pub notes_count: u32,
    /// Non whitespace characters across slide text.
    pub non_whitespace_chars: u32,
    /// Painted slide canvases.
    pub slides: Vec<SlideCanvas>,
    /// Chart scenes that replaced a placeholder frame.
    pub charts: Vec<ChartSceneInfo>,
    /// Import and view issues.
    pub issues: Vec<OfficeIssue>,
}

/// Body advances used for the measured slide wrapping.
struct HostMetrics;

impl document_formats::import::pptx::TextMeasurer for HostMetrics {
    fn advance(&self, character: char, font_px: f64) -> f64 {
        let key = document_fonts::BodyFaceKey {
            family: document_fonts::BodyFamily::Serif,
            style: document_fonts::BodyFaceStyle::Regular,
        };
        match crate::assets::body_metrics().advance_em(key, character) {
            Some(em) => em * font_px,
            None if character.is_ascii() => font_px * 0.55,
            None => font_px,
        }
    }
}

/// Vega-Lite mark that paints the declared PowerPoint chart family.
fn chart_mark(kind: Option<&str>) -> Option<&'static str> {
    match kind.unwrap_or_default() {
        "barChart" | "bar3DChart" => Some("bar"),
        "lineChart" | "line3DChart" => Some("line"),
        "areaChart" | "area3DChart" => Some("area"),
        _ => None,
    }
}

/// Builds the Vega-Lite payload one slide chart carries from its cached data.
fn chart_spec(
    chart: &document_formats::import::pptx::ChartRef,
    width_px: f64,
    height_px: f64,
) -> Option<String> {
    let mark = chart_mark(chart.kind.as_deref())?;
    let series = chart.series.first()?;
    if chart.categories.len() != series.values.len() {
        return None;
    }
    let values: Vec<serde_json::Value> = chart
        .categories
        .iter()
        .zip(&series.values)
        .map(|(category, value)| serde_json::json!({ "category": category, "value": value }))
        .collect();
    Some(
        serde_json::json!({
            "title": series.name.clone(),
            "width": width_px,
            "height": height_px,
            "mark": mark,
            "data": { "values": values },
            "encoding": {
                "x": { "field": "category", "type": "nominal" },
                "y": { "field": "value", "type": "quantitative" }
            }
        })
        .to_string(),
    )
}

/// Lowers every chart frame into a scene and returns the SVG placed on it.
fn chart_scenes(
    presentation: &document_formats::import::pptx::PptxPresentation,
) -> (Vec<(u32, u32, String)>, Vec<ChartSceneInfo>) {
    let mut fragments = Vec::new();
    let mut scenes = Vec::new();
    for slide in &presentation.slides {
        if slide.hidden {
            continue;
        }
        for shape in &slide.shapes {
            let Some(chart) = &shape.chart else { continue };
            let Some(transform) = shape.transform else { continue };
            let width_px = document_formats::import::pptx::emu_to_px(transform.cx);
            let height_px = document_formats::import::pptx::emu_to_px(transform.cy);
            let spec = chart_spec(chart, width_px, height_px);
            let Some(spec) = spec else {
                scenes.push(ChartSceneInfo {
                    slide: slide.index,
                    shape: shape.id,
                    kind: chart.kind.clone(),
                    node_count: 0,
                    width_px,
                    height_px,
                    svg: String::new(),
                    error: Some(format!("chart family `{}` has no scene lowering", chart.kind.clone().unwrap_or_default())),
                });
                continue;
            };
            match plotter_chart::vega_lite::lower(&spec) {
                Ok(lowered) => match lowered.to_svg_with_hooks(
                    &format!("panduck-slide{}-chart{}", slide.index, shape.id),
                    "panduck-chart",
                ) {
                    Ok(svg) => {
                        let x = document_formats::import::pptx::emu_to_px(transform.x);
                        let y = document_formats::import::pptx::emu_to_px(transform.y);
                        let fragment = format!("<g transform=\"translate({x:.1},{y:.1})\">{svg}</g>");
                        scenes.push(ChartSceneInfo {
                            slide: slide.index,
                            shape: shape.id,
                            kind: chart.kind.clone(),
                            node_count: lowered.scene.nodes.len() as u32,
                            width_px: lowered.scene.size.width,
                            height_px: lowered.scene.size.height,
                            svg: fragment.clone(),
                            error: None,
                        });
                        fragments.push((slide.index, shape.id, fragment));
                    }
                    Err(error) => scenes.push(ChartSceneInfo {
                        slide: slide.index,
                        shape: shape.id,
                        kind: chart.kind.clone(),
                        node_count: 0,
                        width_px,
                        height_px,
                        svg: String::new(),
                        error: Some(error.to_string()),
                    }),
                },
                Err(error) => scenes.push(ChartSceneInfo {
                    slide: slide.index,
                    shape: shape.id,
                    kind: chart.kind.clone(),
                    node_count: 0,
                    width_px,
                    height_px,
                    svg: String::new(),
                    error: Some(error.message().to_string()),
                }),
            }
        }
    }
    (fragments, scenes)
}

/// Renders a PPTX package into fixed canvas slide views.
///
/// Charts are lowered from their cached categories and values into plotter
/// scenes, so the placeholder frame is replaced by the painted chart. Text is
/// wrapped with the host body advances rather than the em estimate.
#[napi]
pub fn render_slides(label: String, bytes: Buffer) -> Result<PresentationRenderResponse> {
    let presentation = document_formats::import::pptx::import_pptx_bytes(&label, &bytes)
        .map_err(|error| Error::from_reason(error.to_string()))?;
    let (fragments, charts) = chart_scenes(&presentation);
    let render = document_formats::import::pptx::render_presentation_with(
        &presentation,
        &document_formats::import::pptx::RenderOptions::default(),
        &HostMetrics,
        &fragments,
    );
    let stats = presentation.stats;
    let mut issues: Vec<OfficeIssue> = presentation.issues.iter().map(OfficeIssue::pptx).collect();
    issues.extend(render.issues.iter().map(OfficeIssue::pptx));
    Ok(PresentationRenderResponse {
        html: render.html,
        slide_count: stats.slide_count,
        shape_count: stats.shape_count,
        image_count: stats.image_count,
        table_count: stats.table_count,
        chart_count: stats.chart_count,
        notes_count: stats.notes_count,
        non_whitespace_chars: stats.non_whitespace_chars,
        slides: render
            .views
            .iter()
            .map(|view| SlideCanvas {
                index: view.index,
                width_px: view.width_px,
                height_px: view.height_px,
                svg: view.svg.clone(),
                deviation_count: view.deviations.len() as u32,
            })
            .collect(),
        charts,
        issues: render.issues.iter().map(OfficeIssue::pptx).collect(),
    })
}

/// Line breaking of one slide paragraph under both measurers.
#[napi(object)]
pub struct WrapProbe {
    /// One based slide number.
    pub slide: u32,
    /// Shape id the paragraph belongs to.
    pub shape_id: u32,
    /// Shape name.
    pub shape_name: String,
    /// Zero based paragraph index inside the shape.
    pub paragraph: u32,
    /// Declared or inherited font size in points.
    pub font_size_pt: f64,
    /// Available text width in CSS pixels.
    pub width_px: f64,
    /// Lines produced by the em estimate.
    pub estimated_lines: Vec<String>,
    /// Lines produced by the host advances.
    pub measured_lines: Vec<String>,
}

/// Compares estimated and measured slide text wrapping per paragraph.
#[napi]
pub fn probe_slide_text_wrap(label: String, bytes: Buffer) -> Result<Vec<WrapProbe>> {
    let presentation = document_formats::import::pptx::import_pptx_bytes(&label, &bytes)
        .map_err(|error| Error::from_reason(error.to_string()))?;
    let estimated = document_formats::import::pptx::text_wrap_probe(
        &presentation,
        &document_formats::import::pptx::EstimatedMetrics,
    );
    let measured = document_formats::import::pptx::text_wrap_probe(&presentation, &HostMetrics);
    Ok(estimated
        .iter()
        .zip(&measured)
        .map(|(estimate, measure)| WrapProbe {
            slide: estimate.slide,
            shape_id: estimate.shape_id,
            shape_name: estimate.shape_name.clone(),
            paragraph: estimate.paragraph,
            font_size_pt: estimate.font_size_pt,
            width_px: estimate.width_px,
            estimated_lines: estimate.lines.clone(),
            measured_lines: measure.lines.clone(),
        })
        .collect())
}

#[cfg(test)]
mod diagnostic_catalog {
    use std::collections::BTreeSet;

    /// Codes the shared catalog publishes, which is the only legal source for a
    /// diagnostic this binding emits.
    fn catalog_codes() -> BTreeSet<String> {
        plotter_types::code_docs()
            .into_iter()
            .map(|doc| doc.code)
            .chain(plotter_types::cases().into_iter().map(|case| case.code))
            .chain(plotter_types::matrix().into_iter().filter_map(|row| row.code))
            .collect()
    }

    /// Every chart diagnostic has to resolve in the shared catalog, so a
    /// consumer matching on codes never sees a spelling this binding invented.
    #[test]
    fn chart_diagnostics_name_catalog_codes() {
        let catalog = catalog_codes();
        let specs = [
            // Supported spec: renders without diagnostics.
            r#"{"mark":"point","data":{"values":[{"x":1,"y":2}]},"encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
            // Unsupported mark, refused during lowering.
            r#"{"mark":"rule","data":{"values":[{"x":1,"y":2}]},"encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
            // Unknown field, refused during lowering.
            r#"{"mark":"point","data":{"values":[{"x":1,"y":2}]},"encoding":{"x":{"field":"nope","type":"quantitative"}}}"#,
            // Out-of-range opacity, refused while the scene serializes.
            r#"{"mark":{"type":"point","opacity":2},"data":{"values":[{"x":1,"y":2}]},"encoding":{"x":{"field":"x","type":"quantitative"},"y":{"field":"y","type":"quantitative"}}}"#,
            // Body that is not JSON.
            "{",
        ];
        let mut offenders = Vec::new();
        for spec in specs {
            for diagnostic in super::render_chart(spec.to_string()).diagnostics {
                if !catalog.contains(diagnostic.code.as_str()) {
                    offenders.push(format!("{}: {}", diagnostic.severity, diagnostic.code));
                }
            }
        }
        assert!(offenders.is_empty(), "chart diagnostics outside the shared catalog: {offenders:?}");
    }

    /// The scene codes the SVG serializer reports on the chart path are
    /// catalog rows, so that error branch cannot emit a wild code either.
    #[test]
    fn chart_scene_serialization_codes_are_catalog_backed() {
        let catalog = catalog_codes();
        for code in [
            plotter_types::codes::INVALID_GEOMETRY,
            plotter_types::codes::INVALID_STYLE,
            plotter_types::codes::INVALID_TRANSFORM,
            plotter_types::codes::SERIALIZATION_FAILED,
        ] {
            assert!(catalog.contains(code), "scene code `{code}` is missing from the shared catalog");
        }
    }
}
