use std::collections::{HashMap, HashSet};

use notedown_ir::{Block, DocumentGraph, SemanticStatus};
use quick_xml::events::Event;
use quick_xml::name::LocalName;
use quick_xml::Reader;

/// Resolved footnote bodies keyed by `w:footnote` id.
pub type FootnoteCatalog = HashMap<u32, String>;

const FOOTNOTES_XML: &str = "word/footnotes.xml";

/// Part path for WordprocessingML footnote definitions.
pub fn footnotes_part_path() -> &'static str {
    FOOTNOTES_XML
}

/// Parses `word/footnotes.xml` into plain-text footnote bodies.
pub fn parse_footnotes_xml_lossy(xml: &[u8]) -> FootnoteCatalog {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut catalog = FootnoteCatalog::new();
    let mut current_id: Option<u32> = None;
    let mut in_footnote = false;
    let mut in_paragraph = false;
    let mut in_run = false;
    let mut paragraph = String::new();
    let mut body = String::new();

    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) => {
                let local = tag.local_name();
                if is_local(local, b"footnote") {
                    current_id = u32_attribute(&tag, b"id");
                    in_footnote = current_id.is_some_and(|id| id > 0);
                    body.clear();
                    paragraph.clear();
                } else if in_footnote && is_local(local, b"p") {
                    in_paragraph = true;
                    paragraph.clear();
                } else if in_footnote && in_paragraph && is_local(local, b"r") {
                    in_run = true;
                }
            }
            Event::Empty(tag) => {
                let local = tag.local_name();
                if in_footnote && in_paragraph && in_run && is_local(local, b"tab") {
                    paragraph.push('\t');
                } else if in_footnote && in_paragraph && in_run && is_local(local, b"br") {
                    paragraph.push('\n');
                }
            }
            Event::Text(text) if in_footnote && in_run => {
                if let Ok(decoded) = text.unescape() {
                    paragraph.push_str(&decoded);
                }
            }
            Event::End(tag) => {
                let local = tag.local_name();
                if is_local(local, b"r") {
                    in_run = false;
                } else if is_local(local, b"p") && in_footnote {
                    in_paragraph = false;
                    append_paragraph(&mut body, &paragraph);
                } else if is_local(local, b"footnote") && in_footnote {
                    if let Some(id) = current_id {
                        let trimmed = body.trim();
                        if !trimmed.is_empty() {
                            catalog.insert(id, trimmed.to_string());
                        }
                    }
                    in_footnote = false;
                    current_id = None;
                    body.clear();
                    paragraph.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    catalog
}

/// Appends GFM footnote definition blocks for referenced ids with resolved bodies.
pub fn append_footnote_definitions(
    graph: &mut DocumentGraph,
    catalog: &FootnoteCatalog,
    referenced: &HashSet<u32>,
) {
    let mut ids: Vec<u32> = referenced
        .iter()
        .filter(|id| catalog.contains_key(id))
        .copied()
        .collect();
    ids.sort_unstable();
    for id in ids {
        let body = catalog.get(&id).expect("filtered above");
        graph.push_block(Block::Opaque {
            kind: "footnote_definition".into(),
            payload_hint: format!("[^{}]: {}", id, body),
            status: SemanticStatus::Resolved,
        });
    }
}

fn append_paragraph(body: &mut String, paragraph: &str) {
    let trimmed = paragraph.trim();
    if trimmed.is_empty() {
        return;
    }
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(trimmed);
}

fn is_local(name: LocalName, local: &[u8]) -> bool {
    name.as_ref() == local
}

fn u32_attribute(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<u32> {
    attribute_value(tag, local)?.parse().ok()
}

fn attribute_value(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<String> {
    tag.attributes()
        .filter_map(|attribute| attribute.ok())
        .find(|attribute| attribute.key.local_name().as_ref() == local)
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
}
