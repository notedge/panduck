use std::collections::{HashMap, HashSet};

use notedown_ir::{
    Asset, AssetId, AssetKind, Block, DocumentGraph, DocumentId, Inline, ListItem, LossMarker,
    SemanticStatus,
};
use crate::footnotes::FootnoteCatalog;
use crate::numbering::NumberingCatalog;
use crate::table::{append_cell_paragraph, push_table_block, TableState};
use panduck_types::{AdapterError, Result};
use quick_xml::events::Event;
use quick_xml::name::LocalName;
use quick_xml::Reader;

/// Parses `word/document.xml` body content into a flat `DocumentGraph`.
pub fn parse_document_xml(
    xml: &[u8],
    rels: &HashMap<String, String>,
    numbering: &NumberingCatalog,
    footnotes: &FootnoteCatalog,
    graph: &mut DocumentGraph,
) -> Result<HashSet<u32>> {
    let mut referenced_footnotes = HashSet::new();
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
    let mut in_num_pr = false;
    let mut in_r_pr = false;
    let mut body = BodyState::default();
    let mut in_table = false;
    let mut in_cell = false;
    let mut table = None::<TableState>;

    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(tag) => {
                let local = tag.local_name();
                if is_local(local, b"body") {
                    in_body = true;
                } else if in_body && is_local(local, b"tbl") {
                    flush_pending_list(graph, &mut body, numbering);
                    in_table = true;
                    table = Some(TableState::default());
                } else if in_table && is_local(local, b"tr") {
                    if let Some(table) = table.as_mut() {
                        table.current_row = Vec::new();
                    }
                } else if in_table && is_local(local, b"tc") {
                    in_cell = true;
                    if let Some(table) = table.as_mut() {
                        table.current_cell = Vec::new();
                    }
                } else if in_body && is_local(local, b"p") {
                    in_paragraph = true;
                    paragraph = ParagraphState::default();
                } else if in_paragraph && is_local(local, b"pPr") {
                    in_p_pr = true;
                } else if in_p_pr && is_local(local, b"pStyle") {
                    paragraph.style = style_attribute(&tag);
                } else if in_p_pr && is_local(local, b"numPr") {
                    paragraph.is_list_item = true;
                    in_num_pr = true;
                } else if in_num_pr && is_local(local, b"numId") {
                    paragraph.num_id = u32_attribute(&tag, b"val");
                } else if in_num_pr && is_local(local, b"ilvl") {
                    paragraph.ilvl = u32_attribute(&tag, b"val");
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
                } else if in_paragraph && is_local(local, b"docPr") {
                    paragraph.pending_image_alt = description_attribute(&tag);
                } else if in_paragraph && is_local(local, b"blip") {
                    if let Some(rel_id) = embed_relationship_id(&tag) {
                        paragraph.push_image(&rel_id, rels, graph);
                    }
                } else if in_paragraph && is_local(local, b"footnoteReference") {
                    push_footnote_reference(
                        graph,
                        &mut paragraph,
                        &mut hyperlink,
                        footnotes,
                        &mut referenced_footnotes,
                        u32_attribute(&tag, b"id"),
                    );
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
                    flush_pending_list(graph, &mut body, numbering);
                    in_body = false;
                } else if is_local(local, b"pPr") {
                    in_p_pr = false;
                    in_num_pr = false;
                } else if is_local(local, b"numPr") {
                    in_num_pr = false;
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
                    if in_cell {
                        if let Some(table) = table.as_mut() {
                            if let Some(content) = paragraph_inlines(&paragraph) {
                                append_cell_paragraph(&mut table.current_cell, &content);
                            }
                        }
                    } else if !in_table {
                        finish_paragraph(graph, &mut body, &paragraph, numbering);
                    }
                    in_paragraph = false;
                    paragraph = ParagraphState::default();
                } else if in_table && is_local(local, b"tc") {
                    if let Some(table) = table.as_mut() {
                        table.current_row.push(table.current_cell.clone());
                        table.current_cell.clear();
                    }
                    in_cell = false;
                } else if in_table && is_local(local, b"tr") {
                    if let Some(table) = table.as_mut() {
                        if !table.current_row.is_empty() {
                            table.rows.push(table.current_row.clone());
                        }
                        table.current_row.clear();
                    }
                } else if in_table && is_local(local, b"tbl") {
                    if let Some(table_state) = table.take() {
                        push_table_block(graph, table_state);
                    }
                    in_table = false;
                    in_cell = false;
                }
            }
            Event::Empty(tag) => {
                let local = tag.local_name();
                if in_p_pr && is_local(local, b"pStyle") {
                    paragraph.style = style_attribute(&tag);
                } else if in_p_pr && is_local(local, b"numPr") {
                    paragraph.is_list_item = true;
                    in_num_pr = true;
                } else if in_num_pr && is_local(local, b"numId") {
                    paragraph.num_id = u32_attribute(&tag, b"val");
                } else if in_num_pr && is_local(local, b"ilvl") {
                    paragraph.ilvl = u32_attribute(&tag, b"val");
                } else if in_r_pr && is_local(local, b"b") {
                    run.bold = true;
                } else if in_r_pr && is_local(local, b"i") {
                    run.italic = true;
                } else if in_run && is_local(local, b"tab") {
                    run.text.push('\t');
                } else if in_run && is_local(local, b"br") {
                    run.text.push('\n');
                } else if in_paragraph && is_local(local, b"docPr") {
                    paragraph.pending_image_alt = description_attribute(&tag);
                } else if in_paragraph && is_local(local, b"blip") {
                    if let Some(rel_id) = embed_relationship_id(&tag) {
                        paragraph.push_image(&rel_id, rels, graph);
                    }
                } else if in_paragraph && is_local(local, b"footnoteReference") {
                    push_footnote_reference(
                        graph,
                        &mut paragraph,
                        &mut hyperlink,
                        footnotes,
                        &mut referenced_footnotes,
                        u32_attribute(&tag, b"id"),
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    flush_pending_list(graph, &mut body, numbering);

    if graph.blocks.is_empty() {
        graph.push_loss(LossMarker {
            code: "reader.docx.empty_body".into(),
            message: "word/document.xml contained no paragraphs".into(),
            status: SemanticStatus::Partial,
        });
    }

    Ok(referenced_footnotes)
}

#[derive(Debug, Default)]
struct ParagraphState {
    style: Option<String>,
    inlines: Vec<Inline>,
    pending_image_alt: Option<String>,
    is_list_item: bool,
    num_id: Option<u32>,
    ilvl: Option<u32>,
}

#[derive(Debug, Default)]
struct BodyState {
    pending_list: Option<PendingListState>,
}

#[derive(Debug)]
struct PendingListState {
    num_id: Option<u32>,
    ordered: Option<bool>,
    items: Vec<ListItem>,
}

impl Default for PendingListState {
    fn default() -> Self {
        Self {
            num_id: None,
            ordered: None,
            items: Vec::new(),
        }
    }
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

    fn push_image(
        &mut self,
        rel_id: &str,
        rels: &HashMap<String, String>,
        graph: &mut DocumentGraph,
    ) {
        let alt = self.pending_image_alt.clone().unwrap_or_default();
        self.pending_image_alt = None;
        let target = rels.get(rel_id);
        if let Some(target) = target {
            let asset_id = AssetId(graph.assets.len() as u64 + 1);
            graph.push_asset(Asset {
                id: asset_id,
                kind: AssetKind::Image,
                content_identity: None,
                source: Some(target.clone()),
                media_type: media_type_for(target),
                status: SemanticStatus::Resolved,
            });
            push_inline(
                &mut self.inlines,
                Inline::Styled {
                    style: "image".into(),
                    children: vec![
                        Inline::Text { text: alt },
                        Inline::Text { text: target.clone() },
                    ],
                },
            );
            return;
        }
        graph.push_loss(LossMarker {
            code: "reader.docx.unresolved_image".into(),
            message: format!("image relationship {rel_id} could not be resolved"),
            status: SemanticStatus::Unresolved,
        });
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

fn push_footnote_reference(
    graph: &mut DocumentGraph,
    paragraph: &mut ParagraphState,
    hyperlink: &mut Option<HyperlinkState>,
    footnotes: &FootnoteCatalog,
    referenced: &mut HashSet<u32>,
    id: Option<u32>,
) {
    let label = id.map(|value| value.to_string()).unwrap_or_else(|| "?".to_string());
    let inline = Inline::Text {
        text: format!("[^{}]", label),
    };
    if let Some(link) = hyperlink.as_mut() {
        push_inline(&mut link.inlines, inline);
    } else {
        push_inline(&mut paragraph.inlines, inline);
    }
    if let Some(id) = id {
        referenced.insert(id);
        if !footnotes.contains_key(&id) {
            graph.push_loss(LossMarker {
                code: "reader.docx.footnote_body".into(),
                message: format!("footnote {label} body is not resolved yet"),
                status: SemanticStatus::Unresolved,
            });
        }
    } else {
        graph.push_loss(LossMarker {
            code: "reader.docx.footnote_body".into(),
            message: "footnote reference is missing an id".into(),
            status: SemanticStatus::Unresolved,
        });
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
    attribute_value(tag, b"id")
}

fn embed_relationship_id(tag: &quick_xml::events::BytesStart) -> Option<String> {
    attribute_value(tag, b"embed")
}

fn description_attribute(tag: &quick_xml::events::BytesStart) -> Option<String> {
    attribute_value(tag, b"descr")
}

fn attribute_value(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<String> {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == local)
        .and_then(|attr| attr.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn media_type_for(path: &str) -> Option<String> {
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png".into()),
        "jpg" | "jpeg" => Some("image/jpeg".into()),
        "gif" => Some("image/gif".into()),
        "webp" => Some("image/webp".into()),
        "svg" => Some("image/svg+xml".into()),
        _ => None,
    }
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

fn finish_paragraph(
    graph: &mut DocumentGraph,
    body: &mut BodyState,
    paragraph: &ParagraphState,
    numbering: &NumberingCatalog,
) {
    if paragraph.is_list_item {
        if let Some(content) = paragraph_inlines(paragraph) {
            let item = ListItem {
                content,
                children: Vec::new(),
            };
            let ordered = list_marker_ordered(paragraph, numbering);
            match &mut body.pending_list {
                Some(list) if list.num_id == paragraph.num_id => list.items.push(item),
                Some(_) => {
                    flush_pending_list(graph, body, numbering);
                    body.pending_list = Some(PendingListState {
                        num_id: paragraph.num_id,
                        ordered,
                        items: vec![item],
                    });
                }
                None => {
                    body.pending_list = Some(PendingListState {
                        num_id: paragraph.num_id,
                        ordered,
                        items: vec![item],
                    });
                }
            }
        }
        return;
    }
    flush_pending_list(graph, body, numbering);
    push_paragraph(graph, paragraph);
}

fn list_marker_ordered(paragraph: &ParagraphState, numbering: &NumberingCatalog) -> Option<bool> {
    match (paragraph.num_id, paragraph.ilvl) {
        (Some(num_id), ilvl) => numbering.is_ordered(num_id, ilvl.unwrap_or(0)),
        _ => None,
    }
}

fn flush_pending_list(
    graph: &mut DocumentGraph,
    body: &mut BodyState,
    numbering: &NumberingCatalog,
) {
    let Some(list) = body.pending_list.take() else {
        return;
    };
    if list.items.is_empty() {
        return;
    }
    let resolved = list
        .ordered
        .or_else(|| list.num_id.and_then(|num_id| numbering.is_ordered(num_id, 0)));
    let ordered = resolved.unwrap_or(false);
    if resolved.is_none() {
        graph.push_loss(LossMarker {
            code: "reader.docx.numbering_unresolved".into(),
            message: "list marker style inferred without word/numbering.xml".into(),
            status: SemanticStatus::Partial,
        });
    }
    graph.push_block(Block::List {
        ordered,
        items: list.items,
    });
}

fn u32_attribute(tag: &quick_xml::events::BytesStart, local: &[u8]) -> Option<u32> {
    tag.attributes()
        .filter_map(|attr| attr.ok())
        .find(|attr| attr.key.local_name().as_ref() == local)
        .and_then(|attr| attr.unescape_value().ok())
        .and_then(|value| value.parse().ok())
}

fn paragraph_inlines(paragraph: &ParagraphState) -> Option<Vec<Inline>> {
    let trimmed = paragraph
        .inlines
        .iter()
        .map(inline_plain_text)
        .collect::<String>()
        .trim()
        .to_string();
    if trimmed.is_empty() {
        return None;
    }
    if paragraph.inlines.is_empty() {
        Some(vec![Inline::Text { text: trimmed }])
    } else {
        Some(paragraph.inlines.clone())
    }
}

fn push_paragraph(graph: &mut DocumentGraph, paragraph: &ParagraphState) {
    let Some(inlines) = paragraph_inlines(paragraph) else {
        return;
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
