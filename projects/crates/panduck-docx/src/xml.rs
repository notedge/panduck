use std::collections::HashMap;

use notedown_ir::{Block, DocumentGraph, DocumentId, Inline, LossMarker, SemanticStatus};
use panduck_types::{AdapterError, Result};
use quick_xml::events::Event;
use quick_xml::name::LocalName;
use quick_xml::Reader;

/// Parses `word/document.xml` body content into a flat `DocumentGraph`.
pub fn parse_document_xml(
    xml: &[u8],
    rels: &HashMap<String, String>,
    graph: &mut DocumentGraph,
) -> Result<()> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut in_body = false;
    let mut in_paragraph = false;
    let mut paragraph = ParagraphState::default();
    let mut hyperlink: Option<HyperlinkState> = None;
    let mut in_run = false;
    let mut run = RunState::default();
    let mut in_p_pr = false;
    let mut in_r_pr = false;

    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) => {
                let local = tag.local_name();
                if is_local(local, b"body") {
                    in_body = true;
                } else if in_body && is_local(local, b"p") {
                    in_paragraph = true;
                    paragraph = ParagraphState::default();
                } else if in_paragraph && is_local(local, b"pPr") {
                    in_p_pr = true;
                } else if in_p_pr && is_local(local, b"pStyle") {
                    paragraph.style = style_attribute(&tag);
                } else if in_paragraph && is_local(local, b"hyperlink") {
                    hyperlink = Some(HyperlinkState {
                        rel_id: relationship_id(&tag),
                        inlines: Vec::new(),
                    });
                } else if in_paragraph && is_local(local, b"r") {
                    in_run = true;
                    run = RunState::default();
                } else if in_run && is_local(local, b"rPr") {
                    in_r_pr = true;
                } else if in_r_pr && is_local(local, b"b") {
                    run.bold = bool_attribute(&tag, true);
                } else if in_r_pr && is_local(local, b"i") {
                    run.italic = bool_attribute(&tag, true);
                } else if in_run && is_local(local, b"tab") {
                    run.text.push('\t');
                } else if in_run && is_local(local, b"br") {
                    run.text.push('\n');
                }
            }
            Event::Text(text) if in_run => {
                let decoded = text
                    .unescape()
                    .map_err(|error| AdapterError::adapter("docx", error.to_string()))?;
                run.text.push_str(&decoded);
            }
            Event::End(tag) => {
                let local = tag.local_name();
                if is_local(local, b"body") {
                    in_body = false;
                } else if is_local(local, b"pPr") {
                    in_p_pr = false;
                } else if is_local(local, b"rPr") {
                    in_r_pr = false;
                } else if is_local(local, b"r") && in_run {
                    if let Some(link) = hyperlink.as_mut() {
                        link.push_run(&run);
                    } else {
                        paragraph.push_run(&run);
                    }
                    in_run = false;
                    run = RunState::default();
                } else if is_local(local, b"hyperlink") {
                    if let Some(link) = hyperlink.take() {
                        paragraph.push_hyperlink(&link, rels, graph);
                    }
                } else if is_local(local, b"p") && in_paragraph {
                    push_paragraph(graph, &paragraph);
                    in_paragraph = false;
                    paragraph = ParagraphState::default();
                }
            }
            Event::Empty(tag) => {
                let local = tag.local_name();
                if in_p_pr && is_local(local, b"pStyle") {
                    paragraph.style = style_attribute(&tag);
                } else if in_r_pr && is_local(local, b"b") {
                    run.bold = true;
                } else if in_r_pr && is_local(local, b"i") {
                    run.italic = true;
                } else if in_run && is_local(local, b"tab") {
                    run.text.push('\t');
                } else if in_run && is_local(local, b"br") {
                    run.text.push('\n');
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    if graph.blocks.is_empty() {
        graph.push_loss(LossMarker {
            code: "reader.docx.empty_body".into(),
            message: "word/document.xml contained no paragraphs".into(),
            status: SemanticStatus::Partial,
        });
    }

    Ok(())
}

#[derive(Debug, Default)]
struct ParagraphState {
    style: Option<String>,
    inlines: Vec<Inline>,
}

impl ParagraphState {
    fn push_run(&mut self, run: &RunState) {
        if run.text.is_empty() {
            return;
        }
        let inline = run.inline();
        push_inline(&mut self.inlines, inline);
    }

    fn push_hyperlink(
        &mut self,
        link: &HyperlinkState,
        rels: &HashMap<String, String>,
        graph: &mut DocumentGraph,
    ) {
        let display = link
            .inlines
            .iter()
            .map(inline_plain_text)
            .collect::<String>();
        if display.is_empty() {
            return;
        }
        let url = link.rel_id.as_ref().and_then(|id| rels.get(id));
        if let Some(url) = url {
            push_inline(
                &mut self.inlines,
                Inline::Styled {
                    style: "link".into(),
                    children: vec![
                        Inline::Text { text: display },
                        Inline::Text { text: url.clone() },
                    ],
                },
            );
            return;
        }
        graph.push_loss(LossMarker {
            code: "reader.docx.unresolved_hyperlink".into(),
            message: format!(
                "hyperlink relationship {:?} could not be resolved",
                link.rel_id
            ),
            status: SemanticStatus::Unresolved,
        });
        for inline in &link.inlines {
            push_inline(&mut self.inlines, inline.clone());
        }
    }
}

#[derive(Debug, Default)]
struct HyperlinkState {
    rel_id: Option<String>,
    inlines: Vec<Inline>,
}

impl HyperlinkState {
    fn push_run(&mut self, run: &RunState) {
        if run.text.is_empty() {
            return;
        }
        push_inline(&mut self.inlines, run.inline());
    }
}

#[derive(Debug, Default)]
struct RunState {
    bold: bool,
    italic: bool,
    text: String,
}

impl RunState {
    fn inline(&self) -> Inline {
        let text = Inline::Text {
            text: self.text.clone(),
        };
        if self.bold && self.italic {
            return Inline::Styled {
                style: "bold".into(),
                children: vec![Inline::Styled {
                    style: "italic".into(),
                    children: vec![text],
                }],
            };
        }
        if self.bold {
            return Inline::Styled {
                style: "bold".into(),
                children: vec![text],
            };
        }
        if self.italic {
            return Inline::Styled {
                style: "italic".into(),
                children: vec![text],
            };
        }
        text
    }
}

fn push_inline(inlines: &mut Vec<Inline>, inline: Inline) {
    if let Some(last) = inlines.last_mut() {
        if merge_text_inline(last, &inline) {
            return;
        }
    }
    inlines.push(inline);
}

fn merge_text_inline(existing: &mut Inline, incoming: &Inline) -> bool {
    match (existing, incoming) {
        (Inline::Text { text: left }, Inline::Text { text: right }) => {
            left.push_str(right);
            true
        }
        _ => false,
    }
}

fn is_local(name: LocalName, local: &[u8]) -> bool {
    name.as_ref() == local
}

fn style_attribute(tag: &quick_xml::events::BytesStart) -> Option<String> {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == b"val")
        .and_then(|attr| attr.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn relationship_id(tag: &quick_xml::events::BytesStart) -> Option<String> {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == b"id")
        .and_then(|attr| attr.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn bool_attribute(tag: &quick_xml::events::BytesStart, default: bool) -> bool {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == b"val")
        .and_then(|attr| attr.unescape_value().ok())
        .map(|value| {
            let value = value.as_ref();
            !matches!(value, "0" | "false" | "off")
        })
        .unwrap_or(default)
}

fn push_paragraph(graph: &mut DocumentGraph, paragraph: &ParagraphState) {
    let trimmed = paragraph
        .inlines
        .iter()
        .map(inline_plain_text)
        .collect::<String>()
        .trim()
        .to_string();
    if trimmed.is_empty() {
        return;
    }
    let inlines = if paragraph.inlines.is_empty() {
        vec![Inline::Text { text: trimmed }]
    } else {
        paragraph.inlines.clone()
    };
    if let Some(level) = heading_level(&paragraph.style) {
        graph.push_block(Block::Section {
            level,
            title: inlines,
            children: Vec::new(),
        });
        return;
    }
    if paragraph.style.is_some() {
        graph.push_loss(LossMarker {
            code: "reader.docx.unmapped_style".into(),
            message: format!("paragraph style {:?} mapped to plain text", paragraph.style),
            status: SemanticStatus::Partial,
        });
    }
    graph.push_block(Block::Paragraph { content: inlines });
}

fn inline_plain_text(inline: &Inline) -> String {
    match inline {
        Inline::Text { text } => text.clone(),
        Inline::InlineCode { text } => text.clone(),
        Inline::Styled { children, .. } => children.iter().map(inline_plain_text).collect(),
        Inline::InlineMath { content, .. } => content.clone(),
        Inline::Reference { display, .. } => display.clone(),
    }
}

fn heading_level(style: &Option<String>) -> Option<u8> {
    let style = style.as_deref()?;
    if let Some(rest) = style.strip_prefix("Heading") {
        if let Ok(level) = rest.parse::<u8>() {
            if (1..=6).contains(&level) {
                return Some(level);
            }
        }
    }
    if style.eq_ignore_ascii_case("Title") {
        return Some(1);
    }
    None
}

/// Creates a new graph with a stable document id derived from the label.
pub fn new_graph(label: &str) -> DocumentGraph {
    DocumentGraph::new(document_id_for(label))
}

fn document_id_for(label: &str) -> DocumentId {
    let mut hash = 1u64;
    for byte in label.bytes() {
        hash = hash * 31 + u64::from(byte);
    }
    DocumentId(hash)
}
