//! Host-side Mermaid rendering through the DaVinci diagram chain.
//!
//! `notedown-formats` lowers explicit Mermaid fences to `Block::Diagram`. This
//! module lowers those blocks with `davinci-diagram` and serializes each
//! resolved display scene to SVG, keyed by the authoring block id, so the HTML
//! writer can embed real vector output instead of the source-only fallback.
//!
//! A block that fails to parse, lowers to a non-positive canvas, or fails scene
//! serialization keeps no scene: the exporter falls back to the visible source
//! and the failure becomes an IR loss marker. Nothing is dropped silently and
//! no diagram geometry is invented on the failure path.

use std::collections::HashMap;

use davinci_diagram::mermaid_to_scene;
use notedown_formats::export::html::RenderedDiagram;
use notedown_ir::{Block, DocumentGraph, LossMarker, NodeId, SemanticStatus};

/// Diagram scenes rendered for the HTML writer plus the losses they caused.
#[derive(Default)]
pub struct RenderedDiagrams {
    /// Inline SVG keyed by the authoring diagram block.
    pub scenes: HashMap<NodeId, RenderedDiagram>,
    /// Loss markers for diagrams whose scene is missing or approximate.
    pub losses: Vec<LossMarker>,
}

/// Render every Mermaid `Block::Diagram` in `graph` to an SVG scene.
///
/// Non-fatal lowering diagnostics (approximations, partially unsupported
/// statements) keep the rendered scene but record `Partial` loss. Fatal
/// failures record the stable `davinci.*` code as `Unsupported` loss and the
/// block keeps the visible source fallback in the exported HTML.
pub fn render_mermaid_diagrams(graph: &DocumentGraph) -> RenderedDiagrams {
    let mut rendered = RenderedDiagrams::default();
    for node in &graph.blocks {
        let Block::Diagram { language, source } = &node.block
        else {
            continue;
        };
        if !language.trim().eq_ignore_ascii_case("mermaid") {
            continue;
        }
        match mermaid_to_scene(source) {
            Ok(lowered) => {
                for diagnostic in &lowered.diagnostics {
                    rendered.losses.push(LossMarker {
                        code: diagnostic.code().as_str().to_string(),
                        message: diagnostic.message().to_string(),
                        status: SemanticStatus::Partial,
                    });
                }
                let size = lowered.scene.size;
                if size.width <= 0.0 || size.height <= 0.0 {
                    rendered.losses.push(LossMarker {
                        code: "panduck.diagram.invalid-canvas".to_string(),
                        message: format!("Mermaid diagram lowered to a non-positive canvas: {size:?}"),
                        status: SemanticStatus::Lossy,
                    });
                    continue;
                }
                match lowered.scene.to_svg_with_hooks(&format!("diagram-{}", node.id.0), "diagram") {
                    Ok(svg) => {
                        rendered.scenes.insert(node.id, RenderedDiagram { svg });
                    }
                    Err(error) => {
                        rendered.losses.push(LossMarker {
                            code: error.code().to_string(),
                            message: error.message().to_string(),
                            status: SemanticStatus::Lossy,
                        });
                    }
                }
            }
            Err(error) => {
                rendered.losses.push(LossMarker {
                    code: error.code().to_string(),
                    message: error.message().to_string(),
                    status: SemanticStatus::Unsupported,
                });
            }
        }
    }
    rendered
}
