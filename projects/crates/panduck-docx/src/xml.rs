use notedown_ir::{Block, DocumentGraph, DocumentId, Inline, LossMarker, SemanticStatus};
use panduck_types::{AdapterError, Result};
use quick_xml::events::Event;
use quick_xml::name::LocalName;
use quick_xml::Reader;

/// Parses `word/document.xml` body content into a flat `DocumentGraph`.
pub fn parse_document_xml(xml: &[u8], graph: &mut DocumentGraph) -> Result<()> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut in_body = false;
    let mut in_paragraph = false;
    let mut in_p_pr = false;
    let mut paragraph_style: Option<String> = None;
    let mut paragraph_text = String::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) => {
                let local = tag.local_name();
                if is_local(local, b"body") {
                    in_body = true;
                } else if in_body && is_local(local, b"p") {
                    in_paragraph = true;
                    paragraph_style = None;
                    paragraph_text.clear();
                } else if in_paragraph && is_local(local, b"pPr") {
                    in_p_pr = true;
                } else if in_p_pr && is_local(local, b"pStyle") {
                    paragraph_style = style_attribute(&tag);
                } else if in_paragraph && is_local(local, b"tab") {
                    paragraph_text.push('\t');
                } else if in_paragraph && is_local(local, b"br") {
                    paragraph_text.push('\n');
                }
            }
            Event::Text(text) if in_paragraph => {
                let decoded = text
                    .unescape()
                    .map_err(|error| AdapterError::adapter("docx", error.to_string()))?;
                paragraph_text.push_str(&decoded);
            }
            Event::End(tag) => {
                let local = tag.local_name();
                if is_local(local, b"body") {
                    in_body = false;
                } else if is_local(local, b"pPr") {
                    in_p_pr = false;
                } else if is_local(local, b"p") && in_paragraph {
                    push_paragraph(graph, &paragraph_style, &paragraph_text);
                    in_paragraph = false;
                }
            }
            Event::Empty(tag) if in_p_pr && is_local(tag.local_name(), b"pStyle") => {
                paragraph_style = style_attribute(&tag);
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

fn push_paragraph(graph: &mut DocumentGraph, style: &Option<String>, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let inlines = vec![Inline::Text { text: trimmed.to_string() }];
    if let Some(level) = heading_level(style) {
        graph.push_block(Block::Section {
            level,
            title: inlines,
            children: Vec::new(),
        });
        return;
    }
    if style.is_some() {
        graph.push_loss(LossMarker {
            code: "reader.docx.unmapped_style".into(),
            message: format!("paragraph style {:?} mapped to plain text", style),
            status: SemanticStatus::Partial,
        });
    }
    graph.push_block(Block::Paragraph { content: inlines });
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
