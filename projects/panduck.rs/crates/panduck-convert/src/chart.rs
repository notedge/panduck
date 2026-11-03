//! Host-side chart rendering through the DaVinci chart kernel.
//!
//! `notedown-formats` lowers explicit `chart`, `vega`, and `vega-lite` fences to
//! `Block::Chart`. This module lowers those blocks with `davinci-chart`'s
//! Vega-Lite frontend and serializes the resolved scene to SVG, keyed by the
//! authoring block id, so the HTML writer can embed real vector output instead
//! of the source-only fallback.
//!
//! Charts deliberately do not share the diagram renderer: `diagram.rs` owns the
//! Mermaid, DOT, and PlantUML layout plans, while chart geometry comes from the
//! chart kernel's own scenes. A block that fails validation or scene
//! serialization keeps no scene, and the failure becomes an IR loss marker
//! anchored to the authored block span.

use std::collections::HashMap;

use davinci_chart::vega_lite;
use notedown_formats::export::html::RenderedDiagram;
use notedown_ir::{Block, DocumentGraph, LossMarker, NodeId, SemanticStatus, SourceSpan};

/// Chart scenes rendered for the HTML writer plus the losses they caused.
#[derive(Default)]
pub struct RenderedCharts {
    /// Inline SVG keyed by the authoring chart block.
    pub scenes: HashMap<NodeId, RenderedDiagram>,
    /// Loss markers for charts whose scene is missing or approximate.
    pub losses: Vec<LossMarker>,
    /// Authored document spans aligned with `losses`.
    pub loss_spans: Vec<Option<SourceSpan>>,
}

impl RenderedCharts {
    fn push_loss(&mut self, marker: LossMarker, span: Option<SourceSpan>) {
        self.losses.push(marker);
        self.loss_spans.push(span);
    }
}

/// Render every `chart`, `vega`, or `vega-lite` `Block::Chart` in `graph` to an
/// SVG scene.
///
/// Non-fatal chart diagnostics (approximated marks, expanded degenerate
/// domains) keep the rendered scene but record `Partial` loss. Fatal
/// validation failures record the stable `davinci.chart.*` code as
/// `Unsupported` loss, and the block keeps the visible source fallback.
pub fn render_charts(graph: &DocumentGraph) -> RenderedCharts {
    let mut rendered = RenderedCharts::default();
    for node in &graph.blocks {
        let Block::Chart { language, source } = &node.block
        else {
            continue;
        };
        if !is_vega_language(language) {
            continue;
        }
        let block_span = graph.block_span(node.id);
        let chart = match vega_lite::lower(source) {
            Ok(chart) => chart,
            Err(error) => {
                rendered.push_loss(
                    LossMarker {
                        code: error.code().to_string(),
                        message: error.message().to_string(),
                        status: SemanticStatus::Unsupported,
                    },
                    block_span,
                );
                continue;
            }
        };
        for diagnostic in &chart.diagnostics {
            rendered.push_loss(
                LossMarker {
                    code: diagnostic.code().as_str().to_string(),
                    message: diagnostic.message().to_string(),
                    status: SemanticStatus::Partial,
                },
                block_span,
            );
        }
        match chart.to_svg() {
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
    rendered
}

fn is_vega_language(language: &str) -> bool {
    let language = language.trim();
    language.eq_ignore_ascii_case("chart")
        || language.eq_ignore_ascii_case("vega")
        || language.eq_ignore_ascii_case("vega-lite")
}
