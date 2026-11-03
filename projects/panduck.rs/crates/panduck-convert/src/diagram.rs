//! Host-side diagram rendering through the DaVinci diagram chain.
//!
//! `notedown-formats` lowers explicit Mermaid, DOT, and PlantUML fences to
//! `Block::Diagram`. This module lowers those blocks with `davinci-diagram` and
//! serializes each resolved display scene to SVG, keyed by the authoring block
//! id, so the HTML writer can embed real vector output instead of the
//! source-only fallback.
//!
//! A block that fails to parse, lowers to a non-positive canvas, or fails scene
//! serialization keeps no scene: the exporter falls back to the visible source
//! and the failure becomes an IR loss marker. Nothing is dropped silently and
//! no diagram geometry is invented on the failure path.

use std::collections::HashMap;

use davinci_diagram::{dot_to_scene, mermaid_to_scene, plantuml_to_scene};
use notedown_formats::export::html::RenderedDiagram;
use notedown_ir::{Block, DocumentGraph, LossMarker, NodeId, SemanticStatus, SourceSpan};

/// Diagram scenes rendered for the HTML writer plus the losses they caused.
#[derive(Default)]
pub struct RenderedDiagrams {
    /// Inline SVG keyed by the authoring diagram block.
    pub scenes: HashMap<NodeId, RenderedDiagram>,
    /// Loss markers for diagrams whose scene is missing or approximate.
    pub losses: Vec<LossMarker>,
    /// Authored document spans aligned with `losses`.
    pub loss_spans: Vec<Option<SourceSpan>>,
}

impl RenderedDiagrams {
    fn push_loss(&mut self, marker: LossMarker, span: Option<SourceSpan>) {
        self.losses.push(marker);
        self.loss_spans.push(span);
    }
}

fn absolute_span(block: Option<SourceSpan>, local: Option<std::ops::Range<usize>>) -> Option<SourceSpan> {
    match (block, local) {
        (Some(block), Some(local)) => SourceSpan::new(block.start as usize + local.start, block.start as usize + local.end),
        _ => block,
    }
}

/// Render every Mermaid, DOT, or PlantUML `Block::Diagram` in `graph` to an SVG
/// scene.
///
/// Non-fatal lowering diagnostics (approximations, partially unsupported
/// statements) keep the rendered scene but record `Partial` loss. Fatal
/// failures record the stable `davinci.*` code as `Unsupported` loss and the
/// block keeps the visible source fallback in the exported HTML.
pub fn render_diagrams(graph: &DocumentGraph) -> RenderedDiagrams {
    let mut rendered = RenderedDiagrams::default();
    for node in &graph.blocks {
        let Block::Diagram { language, source } = &node.block
        else {
            continue;
        };
        let language = language.trim();
        let block_span = graph.block_span(node.id);
        let lowered = if language.eq_ignore_ascii_case("mermaid") {
            mermaid_to_scene(source)
        } else if language.eq_ignore_ascii_case("dot") {
            dot_to_scene(source)
        } else if language.eq_ignore_ascii_case("plantuml") || language.eq_ignore_ascii_case("puml") {
            plantuml_to_scene(source)
        } else {
            continue;
        };
        match lowered {
            Ok(lowered) => {
                for diagnostic in &lowered.diagnostics {
                    rendered.push_loss(
                        LossMarker {
                            code: diagnostic.code().as_str().to_string(),
                            message: diagnostic.message().to_string(),
                            status: SemanticStatus::Partial,
                        },
                        block_span,
                    );
                }
                let size = lowered.scene.size;
                if size.width <= 0.0 || size.height <= 0.0 {
                    rendered.push_loss(
                        LossMarker {
                            code: "panduck.diagram.invalid-canvas".to_string(),
                            message: format!("Diagram lowered to a non-positive canvas: {size:?}"),
                            status: SemanticStatus::Lossy,
                        },
                        block_span,
                    );
                    continue;
                }
                match lowered.scene.to_svg_with_hooks(&format!("diagram-{}", node.id.0), "diagram", &davinci_types::DIAGRAM_THEME) {
                    Ok(svg) => {
                        rendered.scenes.insert(node.id, RenderedDiagram { svg });
                    }
                    Err(error) => {
                        rendered.push_loss(
                            LossMarker {
                                code: error.code().to_string(),
                                message: error.message().to_string(),
                                status: SemanticStatus::Lossy,
                            },
                            block_span,
                        );
                    }
                }
            }
            Err(error) => {
                rendered.push_loss(
                    LossMarker {
                        code: error.code().to_string(),
                        message: error.message().to_string(),
                        status: SemanticStatus::Unsupported,
                    },
                    absolute_span(block_span, error.span()),
                );
            }
        }
    }
    rendered
}
