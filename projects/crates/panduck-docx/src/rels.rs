use std::collections::HashMap;

use panduck_types::{AdapterError, Result};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parses OPC relationship targets keyed by relationship id.
pub fn parse_relationship_targets(xml: &[u8]) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut targets = HashMap::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) | Event::Empty(tag) => {
                if tag.local_name().as_ref() != b"Relationship" {
                    continue;
                }
                let mut id = None;
                let mut target = None;
                for attr in tag.attributes().flatten() {
                    match attr.key.local_name().as_ref() {
                        b"Id" => {
                            id = attr.unescape_value().ok().map(|value| value.into_owned());
                        }
                        b"Target" => {
                            target = attr.unescape_value().ok().map(|value| value.into_owned());
                        }
                        _ => {}
                    }
                }
                if let (Some(id), Some(target)) = (id, target) {
                    targets.insert(id, target);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(targets)
}

/// Reads `word/_rels/document.xml.rels` when present.
pub fn read_document_relationships(
    package: &acorn_docx::OpcPackage,
    budget: &acorn_core::ParseBudget,
) -> Result<HashMap<String, String>> {
    match package.read_part("word/_rels/document.xml.rels", budget) {
        Ok(xml) => parse_relationship_targets(&xml),
        Err(acorn_docx::OpcError::PartNotFound(_)) => Ok(HashMap::new()),
        Err(error) => Err(AdapterError::adapter("docx", error.to_string())),
    }
}
