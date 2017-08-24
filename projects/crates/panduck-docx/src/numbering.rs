use std::collections::HashMap;

use panduck_types::Result;
use quick_xml::events::Event;
use quick_xml::Reader;

/// Resolved list marker styles from `word/numbering.xml`.
#[derive(Debug, Default, Clone)]
pub struct NumberingCatalog {
    abstract_levels: HashMap<u32, HashMap<u32, String>>,
    num_to_abstract: HashMap<u32, u32>,
}

impl NumberingCatalog {
    /// Returns whether the list marker is ordered when numbering metadata is known.
    pub fn is_ordered(&self, num_id: u32, ilvl: u32) -> Option<bool> {
        let abstract_id = self.num_to_abstract.get(&num_id)?;
        let levels = self.abstract_levels.get(abstract_id)?;
        let format = levels.get(&ilvl)?;
        Some(num_fmt_is_ordered(format))
    }
}

/// Parses `word/numbering.xml` into a lookup table for list marker styles.
pub fn parse_numbering_xml(xml: &[u8]) -> Result<NumberingCatalog> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut catalog = NumberingCatalog::default();
    let mut current_abstract: Option<u32> = None;
    let mut current_ilvl: Option<u32> = None;
    let mut current_num_id: Option<u32> = None;

    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) => {
                let local = tag.local_name();
                match local.as_ref() {
                    b"abstractNum" => {
                        current_abstract = u32_attribute(&tag, b"abstractNumId");
                        current_ilvl = None;
                    }
                    b"lvl" if current_abstract.is_some() => {
                        current_ilvl = u32_attribute(&tag, b"ilvl");
                    }
                    b"numFmt" if current_abstract.is_some() && current_ilvl.is_some() => {
                        record_num_fmt(&mut catalog, current_abstract, current_ilvl, &tag);
                    }
                    b"num" => {
                        current_num_id = u32_attribute(&tag, b"numId");
                    }
                    b"abstractNumId" if current_num_id.is_some() => {
                        if let (Some(num_id), Some(abstract_id)) =
                            (current_num_id, w_val_attribute(&tag))
                        {
                            if let Ok(abstract_id) = abstract_id.parse::<u32>() {
                                catalog.num_to_abstract.insert(num_id, abstract_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::Empty(tag) => {
                let local = tag.local_name();
                match local.as_ref() {
                    b"numFmt" if current_abstract.is_some() && current_ilvl.is_some() => {
                        record_num_fmt(&mut catalog, current_abstract, current_ilvl, &tag);
                    }
                    b"abstractNumId" if current_num_id.is_some() => {
                        if let (Some(num_id), Some(abstract_id)) =
                            (current_num_id, w_val_attribute(&tag))
                        {
                            if let Ok(abstract_id) = abstract_id.parse::<u32>() {
                                catalog.num_to_abstract.insert(num_id, abstract_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::End(tag) => {
                let local = tag.local_name();
                match local.as_ref() {
                    b"abstractNum" => {
                        current_abstract = None;
                        current_ilvl = None;
                    }
                    b"lvl" => current_ilvl = None,
                    b"num" => current_num_id = None,
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(catalog)
}

fn record_num_fmt(
    catalog: &mut NumberingCatalog,
    abstract_id: Option<u32>,
    ilvl: Option<u32>,
    tag: &quick_xml::events::BytesStart,
) {
    if let (Some(abstract_id), Some(ilvl)) = (abstract_id, ilvl) {
        if let Some(format) = w_val_attribute(tag) {
            catalog
                .abstract_levels
                .entry(abstract_id)
                .or_default()
                .insert(ilvl, format);
        }
    }
}

fn num_fmt_is_ordered(format: &str) -> bool {
    !matches!(
        format,
        "bullet" | "none" | "chart" | "image" | "customBullet"
    )
}

fn u32_attribute(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<u32> {
    attribute_value(tag, local)?.parse().ok()
}

fn w_val_attribute(tag: &quick_xml::events::BytesStart) -> Option<String> {
    attribute_value(tag, b"val")
}

fn attribute_value(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<String> {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == local)
        .and_then(|attr| attr.unescape_value().ok())
        .map(|value| value.into_owned())
}

/// Best-effort numbering parse that ignores malformed fragments.
pub fn parse_numbering_xml_lossy(xml: &[u8]) -> NumberingCatalog {
    parse_numbering_xml(xml).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_decimal_numbering_as_ordered() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;
        let catalog = parse_numbering_xml(xml).expect("parse numbering");
        assert_eq!(catalog.is_ordered(1, 0), Some(true));
    }

    #[test]
    fn resolves_bullet_numbering_as_unordered() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="bullet"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="2">
    <w:abstractNumId w:val="1"/>
  </w:num>
</w:numbering>"#;
        let catalog = parse_numbering_xml(xml).expect("parse numbering");
        assert_eq!(catalog.is_ordered(2, 0), Some(false));
    }
}
