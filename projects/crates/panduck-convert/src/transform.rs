//! Executable transform passes over [`DocumentGraph`].
//!
//! The TS-side `ConversionPlan.transforms` author surface declares passes by
//! stable name plus serializable parameters. This module resolves those
//! declarations into real edits on the Notedown IR graph before export:
//!
//! - `replace-text` rewrites `Inline::Text` runs through the notedown-ir
//!   mutation API (`replace_block`).
//! - `remove-blocks` removes blocks whose kind is listed in `targetKinds`
//!   through [`DocumentGraph::walk_mut`] and [`RewriteResult::Remove`], then
//!   deletes the unreachable descendants of the removed blocks.
//! - `normalize-headings` renumbers `Block::Section` levels so top-level
//!   sections start at 1 and every nested section is exactly one level deeper,
//!   clamped to 6.
//! - `resolve-references` fills empty `Inline::Reference` display text from the
//!   referenced block and replaces references to missing nodes with plain text.
//!
//! Every pass produces a serializable report plus structured boost
//! diagnostics. An empty or absent `transforms` array leaves the graph and the
//! report untouched, so conversions without transforms keep the previous
//! behavior.

use std::collections::{HashMap, HashSet};

use boost_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticOrigin, DiagnosticSeverity, Message, MessageArg};
use notedown_ir::{
    AssetId, Block, DocumentGraph, Inline, ListItem, NodeId, RelationEndpoint, RewriteResult, TableRow, Traversal,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Built-in pass name: replace text occurrences inside inline content.
pub const PASS_REPLACE_TEXT: &str = "replace-text";

/// Built-in pass name: remove blocks by block kind.
pub const PASS_REMOVE_BLOCKS: &str = "remove-blocks";

/// Built-in pass name: renumber nested section levels.
pub const PASS_NORMALIZE_HEADINGS: &str = "normalize-headings";

/// Built-in pass name: resolve intra-document references.
pub const PASS_RESOLVE_REFERENCES: &str = "resolve-references";

/// One serializable transform pass declaration from the TS-side plan.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformPassSpec {
    /// Stable pass name used to select a built-in implementation.
    pub name: String,
    /// Optional traversal order. Defaults to bottom-up.
    #[serde(default)]
    pub traversal: Option<String>,
    /// Optional block-kind filter (used by structural passes).
    #[serde(default)]
    pub target_kinds: Vec<String>,
    /// Optional pass-specific parameters. `replace-text` reads `search` and
    /// `replacement` from this object.
    #[serde(default)]
    pub params: Option<Value>,
    /// Optional human-readable description copied into the report.
    #[serde(default)]
    pub description: Option<String>,
}

/// Outcome state of one transform pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformPassStatus {
    /// Pass ran and changed at least one node.
    Applied,
    /// Pass ran but matched nothing.
    Skipped,
    /// No built-in implementation exists for the declared name.
    Unsupported,
    /// The declaration lacked required parameters.
    InvalidParams,
}

/// Serializable report entry for one transform pass.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformPassReport {
    /// Declared pass name.
    pub name: String,
    /// Outcome state.
    pub status: TransformPassStatus,
    /// Number of blocks directly changed or removed by this pass.
    pub changed_nodes: usize,
    /// Number of inline text occurrences replaced (`replace-text`) or
    /// references repaired or replaced (`resolve-references`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_occurrences: Option<usize>,
    /// Number of unreachable descendant blocks swept after removal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swept_nodes: Option<usize>,
    /// Declared block-kind filter, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_kinds: Option<Vec<String>>,
    /// Declared description, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Explanation for unsupported or invalid declarations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Aggregated result of applying every declared transform pass.
#[derive(Debug, Clone, Default)]
pub struct TransformOutcome {
    /// Ordered per-pass report entries.
    pub passes: Vec<TransformPassReport>,
    /// Structured diagnostics produced by pass execution.
    pub diagnostics: Vec<Diagnostic>,
}

impl TransformOutcome {
    /// Whether no pass was declared or executed.
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

/// Apply the `transforms` array carried in the TS-side plan descriptor.
///
/// A missing, empty, or malformed descriptor yields an empty outcome and never
/// fails the conversion. Unknown pass names are reported as `unsupported` and
/// skipped instead of aborting the pipeline.
pub fn apply_transform_passes(graph: &mut DocumentGraph, handlers_json: Option<&str>) -> TransformOutcome {
    let Some(specs) = parse_transform_specs(handlers_json)
    else {
        return TransformOutcome::default();
    };
    let mut outcome = TransformOutcome::default();
    for spec in specs {
        let (report, diagnostics) = run_pass(graph, &spec);
        outcome.diagnostics.extend(diagnostics);
        outcome.passes.push(report);
    }
    outcome
}

fn parse_transform_specs(handlers_json: Option<&str>) -> Option<Vec<TransformPassSpec>> {
    let raw = handlers_json?;
    if raw.trim().is_empty() {
        return None;
    }
    let value = serde_json::from_str::<Value>(raw).ok()?;
    let transforms = value.get("transforms")?.as_array()?;
    Some(transforms.iter().filter_map(|entry| serde_json::from_value(entry.clone()).ok()).collect())
}

fn run_pass(graph: &mut DocumentGraph, spec: &TransformPassSpec) -> (TransformPassReport, Vec<Diagnostic>) {
    match spec.name.as_str() {
        PASS_REPLACE_TEXT => run_replace_text(graph, spec),
        PASS_REMOVE_BLOCKS => run_remove_blocks(graph, spec),
        PASS_NORMALIZE_HEADINGS => run_normalize_headings(graph, spec),
        PASS_RESOLVE_REFERENCES => run_resolve_references(graph, spec),
        _ => {
            let message = format!("no built-in transform pass named \"{}\"", spec.name);
            let diagnostic = pass_diagnostic(
                "panduck.transform.unsupported-pass",
                DiagnosticSeverity::Warning,
                format!("Transform pass \"{}\" is not implemented and was skipped", spec.name),
            )
            .with_note(Message::new("panduck.transform.unsupported-pass.hint").with_fallback(message.clone()));
            (
                TransformPassReport {
                    name: spec.name.clone(),
                    status: TransformPassStatus::Unsupported,
                    changed_nodes: 0,
                    replaced_occurrences: None,
                    swept_nodes: None,
                    target_kinds: normalized_kinds(spec),
                    description: spec.description.clone(),
                    message: Some(message),
                },
                vec![diagnostic],
            )
        }
    }
}

fn run_replace_text(graph: &mut DocumentGraph, spec: &TransformPassSpec) -> (TransformPassReport, Vec<Diagnostic>) {
    let search = spec.params.as_ref().and_then(|params| params.get("search")).and_then(Value::as_str);
    let replacement = spec.params.as_ref().and_then(|params| params.get("replacement")).and_then(Value::as_str);
    let Some(search) = search.filter(|value| !value.is_empty())
    else {
        return invalid_params(spec, "`replace-text` requires a non-empty `params.search` string");
    };
    let replacement = replacement.unwrap_or("");

    let ids: Vec<NodeId> = graph.blocks.iter().map(|node| node.id).collect();
    let mut changed_nodes = 0;
    let mut replaced_occurrences = 0;
    for id in ids {
        let Some(node) = graph.block(id)
        else {
            continue;
        };
        let block = node.block.clone();
        let Some((next_block, occurrences)) = rewrite_block(&block, search, replacement)
        else {
            continue;
        };
        if occurrences == 0 {
            continue;
        }
        if graph.replace_block(id, next_block).is_ok() {
            changed_nodes += 1;
            replaced_occurrences += occurrences;
        }
    }

    let status = if changed_nodes == 0 { TransformPassStatus::Skipped } else { TransformPassStatus::Applied };
    let diagnostics = if changed_nodes == 0 {
        Vec::new()
    }
    else {
        vec![
            pass_diagnostic(
                "panduck.transform.pass-applied",
                DiagnosticSeverity::Info,
                format!("Transform pass \"{}\" replaced {replaced_occurrences} text occurrence(s)", spec.name),
            )
            .with_note(
                Message::new("panduck.transform.pass-applied")
                    .with_arg("pass", MessageArg::Text(spec.name.clone()))
                    .with_arg("changedNodes", MessageArg::U64(changed_nodes as u64))
                    .with_arg("replacedOccurrences", MessageArg::U64(replaced_occurrences as u64)),
            ),
        ]
    };

    (
        TransformPassReport {
            name: spec.name.clone(),
            status,
            changed_nodes,
            replaced_occurrences: Some(replaced_occurrences),
            swept_nodes: None,
            target_kinds: normalized_kinds(spec),
            description: spec.description.clone(),
            message: None,
        },
        diagnostics,
    )
}

fn run_remove_blocks(graph: &mut DocumentGraph, spec: &TransformPassSpec) -> (TransformPassReport, Vec<Diagnostic>) {
    let kinds = normalized_kinds(spec).unwrap_or_default();
    if kinds.is_empty() {
        return invalid_params(spec, "`remove-blocks` requires a non-empty `targetKinds` list");
    }

    // Capture the subtree of every matching block before mutation: removing a
    // section removes its reachable descendants too, rather than promoting
    // them back to document roots.
    let subtrees: HashMap<NodeId, Vec<NodeId>> = graph
        .blocks
        .iter()
        .filter(|node| kinds.iter().any(|kind| block_kind(&node.block).eq_ignore_ascii_case(kind)))
        .map(|node| (node.id, descendants(graph, node.id)))
        .collect();

    let mut removed_nodes = 0usize;
    let order = traversal_of(spec);
    let diagnostics = graph.walk_mut(order, |_id, block, _children| {
        if kinds.iter().any(|kind| block_kind(block).eq_ignore_ascii_case(kind)) {
            removed_nodes += 1;
            RewriteResult::Remove
        }
        else {
            RewriteResult::Keep
        }
    });

    let mut swept_nodes = 0usize;
    for subtree in subtrees.values() {
        for id in subtree {
            if graph.block(*id).is_some() && graph.delete_block(*id).is_ok() {
                swept_nodes += 1;
            }
        }
    }
    if removed_nodes > 0 || swept_nodes > 0 {
        prune_dangling_references(graph);
    }

    let mut collected: Vec<Diagnostic> = diagnostics
        .into_iter()
        .map(|diagnostic| {
            pass_diagnostic(
                "panduck.transform.rewrite-error",
                DiagnosticSeverity::Warning,
                format!("Transform pass \"{}\" could not rewrite a node: {}", spec.name, diagnostic.message),
            )
        })
        .collect();
    if removed_nodes > 0 {
        collected.push(
            pass_diagnostic(
                "panduck.transform.pass-applied",
                DiagnosticSeverity::Info,
                format!("Transform pass \"{}\" removed {removed_nodes} block(s)", spec.name),
            )
            .with_note(
                Message::new("panduck.transform.pass-applied")
                    .with_arg("pass", MessageArg::Text(spec.name.clone()))
                    .with_arg("changedNodes", MessageArg::U64(removed_nodes as u64))
                    .with_arg("sweptNodes", MessageArg::U64(swept_nodes as u64)),
            ),
        );
    }

    let status = if removed_nodes == 0 { TransformPassStatus::Skipped } else { TransformPassStatus::Applied };
    (
        TransformPassReport {
            name: spec.name.clone(),
            status,
            changed_nodes: removed_nodes,
            replaced_occurrences: None,
            swept_nodes: Some(swept_nodes),
            target_kinds: Some(kinds),
            description: spec.description.clone(),
            message: None,
        },
        collected,
    )
}

fn run_normalize_headings(graph: &mut DocumentGraph, spec: &TransformPassSpec) -> (TransformPassReport, Vec<Diagnostic>) {
    // Sections hold their children directly, while lists hold theirs on each
    // item. Treat both as structural containment so nested headings are found
    // from the real document roots.
    let contained: HashSet<NodeId> = graph.blocks.iter().flat_map(|node| child_ids(&node.block)).collect();
    let roots: Vec<NodeId> = graph.blocks.iter().filter(|node| !contained.contains(&node.id)).map(|node| node.id).collect();

    let mut changed_nodes = 0usize;
    for root in roots {
        normalize_heading_level(graph, root, 0, &mut changed_nodes);
    }

    let status = if changed_nodes == 0 { TransformPassStatus::Skipped } else { TransformPassStatus::Applied };
    let diagnostics = if changed_nodes == 0 {
        Vec::new()
    }
    else {
        vec![
            pass_diagnostic(
                "panduck.transform.pass-applied",
                DiagnosticSeverity::Info,
                format!("Transform pass \"{}\" renumbered {changed_nodes} section level(s)", spec.name),
            )
            .with_note(
                Message::new("panduck.transform.pass-applied")
                    .with_arg("pass", MessageArg::Text(spec.name.clone()))
                    .with_arg("changedNodes", MessageArg::U64(changed_nodes as u64)),
            ),
        ]
    };

    (
        TransformPassReport {
            name: spec.name.clone(),
            status,
            changed_nodes,
            replaced_occurrences: None,
            swept_nodes: None,
            target_kinds: normalized_kinds(spec),
            description: spec.description.clone(),
            message: None,
        },
        diagnostics,
    )
}

/// Assign levels to the section `id` and its nested sections.
///
/// `parent_level` is 0 for document roots, so top-level sections resolve to 1,
/// every nested section to `parent + 1`, and levels saturate at 6.
fn normalize_heading_level(graph: &mut DocumentGraph, id: NodeId, parent_level: u8, changed_nodes: &mut usize) {
    let Some(node) = graph.block(id)
    else {
        return;
    };
    let section = match &node.block {
        Block::Section { level, title, children } => Some((*level, title.clone(), children.clone())),
        _ => None,
    };
    let list_children = match &node.block {
        Block::List { items, .. } => Some(items.iter().flat_map(|item| item.children.iter().copied()).collect::<Vec<_>>()),
        _ => None,
    };

    if let Some((level, title, children)) = section {
        let target = (parent_level + 1).min(6);
        if level != target
            && graph.replace_block(id, Block::Section { level: target, title, children: children.clone() }).is_ok()
        {
            *changed_nodes += 1;
        }
        for child in children {
            normalize_heading_level(graph, child, target, changed_nodes);
        }
    }
    else if let Some(children) = list_children {
        // A list item's nested blocks stay at the enclosing section's depth.
        for child in children {
            normalize_heading_level(graph, child, parent_level, changed_nodes);
        }
    }
}

fn run_resolve_references(graph: &mut DocumentGraph, spec: &TransformPassSpec) -> (TransformPassReport, Vec<Diagnostic>) {
    let targets: HashMap<NodeId, String> =
        graph.blocks.iter().map(|node| (node.id, readable_block_text(&node.block))).collect();
    let ids: Vec<NodeId> = graph.blocks.iter().map(|node| node.id).collect();

    let mut changed_nodes = 0usize;
    let mut repaired_references = 0usize;
    let mut diagnostics = Vec::new();
    for id in ids {
        let Some(node) = graph.block(id)
        else {
            continue;
        };
        let block = node.block.clone();
        let Some(next_block) = resolve_block(&block, &targets, &mut repaired_references, &mut diagnostics, &spec.name)
        else {
            continue;
        };
        if graph.replace_block(id, next_block).is_ok() {
            changed_nodes += 1;
        }
    }

    let status = if changed_nodes == 0 { TransformPassStatus::Skipped } else { TransformPassStatus::Applied };
    if changed_nodes > 0 {
        diagnostics.push(
            pass_diagnostic(
                "panduck.transform.pass-applied",
                DiagnosticSeverity::Info,
                format!(
                    "Transform pass \"{}\" resolved {repaired_references} reference(s) in {changed_nodes} block(s)",
                    spec.name
                ),
            )
            .with_note(
                Message::new("panduck.transform.pass-applied")
                    .with_arg("pass", MessageArg::Text(spec.name.clone()))
                    .with_arg("changedNodes", MessageArg::U64(changed_nodes as u64))
                    .with_arg("replacedOccurrences", MessageArg::U64(repaired_references as u64)),
            ),
        );
    }

    (
        TransformPassReport {
            name: spec.name.clone(),
            status,
            changed_nodes,
            replaced_occurrences: Some(repaired_references),
            swept_nodes: None,
            target_kinds: normalized_kinds(spec),
            description: spec.description.clone(),
            message: None,
        },
        diagnostics,
    )
}

fn resolve_block(
    block: &Block,
    targets: &HashMap<NodeId, String>,
    repaired_references: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
    pass_name: &str,
) -> Option<Block> {
    match block {
        Block::Section { level, title, children } => {
            let (title, changed) = resolve_inlines(title, targets, repaired_references, diagnostics, pass_name);
            changed.then(|| Block::Section { level: *level, title, children: children.clone() })
        }
        Block::Paragraph { content } => {
            let (content, changed) = resolve_inlines(content, targets, repaired_references, diagnostics, pass_name);
            changed.then_some(Block::Paragraph { content })
        }
        Block::Quote { content } => {
            let (content, changed) = resolve_inlines(content, targets, repaired_references, diagnostics, pass_name);
            changed.then_some(Block::Quote { content })
        }
        Block::List { ordered, items } => {
            let mut changed = false;
            let items = items
                .iter()
                .map(|item| {
                    let (content, item_changed) =
                        resolve_inlines(&item.content, targets, repaired_references, diagnostics, pass_name);
                    changed |= item_changed;
                    ListItem { content, children: item.children.clone(), number: item.number }
                })
                .collect::<Vec<_>>();
            changed.then(|| Block::List { ordered: *ordered, items })
        }
        // Table captions are already plain strings and cell content is inline
        // prose, so references can still appear inside cells.
        Block::Table { caption, rows } => {
            let mut changed = false;
            let rows = rows
                .iter()
                .map(|row| {
                    let cells = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let (content, cell_changed) =
                                resolve_inlines(&cell.content, targets, repaired_references, diagnostics, pass_name);
                            changed |= cell_changed;
                            notedown_ir::TableCell { content, colspan: cell.colspan, rowspan: cell.rowspan }
                        })
                        .collect::<Vec<_>>();
                    TableRow { is_header: row.is_header, cells }
                })
                .collect::<Vec<_>>();
            changed.then(|| Block::Table { caption: caption.clone(), rows })
        }
        Block::Code { .. } | Block::Math { .. } | Block::Diagram { .. } | Block::Opaque { .. } => None,
    }
}

fn resolve_inlines(
    inlines: &[Inline],
    targets: &HashMap<NodeId, String>,
    repaired_references: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
    pass_name: &str,
) -> (Vec<Inline>, bool) {
    let mut changed = false;
    let mut resolved = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Reference { target, display } => {
                if let Some(text) = targets.get(target) {
                    if display.is_empty() && !text.is_empty() {
                        *repaired_references += 1;
                        changed = true;
                        resolved.push(Inline::Reference { target: *target, display: text.clone() });
                    }
                    else {
                        resolved.push(inline.clone());
                    }
                }
                else {
                    // The target block no longer exists: keep the authored text
                    // as plain content instead of exporting a broken reference.
                    *repaired_references += 1;
                    changed = true;
                    diagnostics.push(dangling_reference_diagnostic(pass_name, *target));
                    resolved.push(Inline::Text { text: display.clone() });
                }
            }
            Inline::Styled { style, children } => {
                let (children, child_changed) = resolve_inlines(children, targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Styled { style: style.clone(), children });
            }
            Inline::Attributed { attributes, children } => {
                let (children, child_changed) = resolve_inlines(children, targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Attributed { attributes: attributes.clone(), children });
            }
            Inline::Link { target, display, title } => {
                let (display, child_changed) = resolve_inlines(display, targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Link { target: target.clone(), display, title: title.clone() });
            }
            Inline::Image { source, alt, title, asset } => {
                let (alt, child_changed) = resolve_inlines(alt, targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Image { source: source.clone(), alt, title: title.clone(), asset: *asset });
            }
            Inline::Note { label, display } => {
                let (display, child_changed) =
                    resolve_optional_inlines(display.as_deref(), targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Note { label: label.clone(), display });
            }
            Inline::Citation { key, display } => {
                let (display, child_changed) =
                    resolve_optional_inlines(display.as_deref(), targets, repaired_references, diagnostics, pass_name);
                changed |= child_changed;
                resolved.push(Inline::Citation { key: key.clone(), display });
            }
            other => resolved.push(other.clone()),
        }
    }
    (resolved, changed)
}

fn resolve_optional_inlines(
    inlines: Option<&[Inline]>,
    targets: &HashMap<NodeId, String>,
    repaired_references: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
    pass_name: &str,
) -> (Option<Vec<Inline>>, bool) {
    match inlines {
        Some(inlines) => {
            let (resolved, changed) = resolve_inlines(inlines, targets, repaired_references, diagnostics, pass_name);
            (Some(resolved), changed)
        }
        None => (None, false),
    }
}

fn dangling_reference_diagnostic(pass_name: &str, target: NodeId) -> Diagnostic {
    pass_diagnostic(
        "panduck.transform.dangling-reference",
        DiagnosticSeverity::Warning,
        format!("Transform pass \"{pass_name}\" replaced a dangling reference to node {} with its display text", target.0),
    )
    .with_note(
        Message::new("panduck.transform.dangling-reference")
            .with_arg("pass", MessageArg::Text(pass_name.to_string()))
            .with_arg("target", MessageArg::U64(target.0)),
    )
}

/// Readable text used to fill an empty reference display.
///
/// Sections use their title, prose blocks their content, lists their items, and
/// tables their caption or cells. Code, math, diagram, and opaque payloads are
/// authored source rather than prose, so they yield no display text.
fn readable_block_text(block: &Block) -> String {
    match block {
        Block::Section { title, .. } => plain_text_of(title),
        Block::Paragraph { content } => plain_text_of(content),
        Block::Quote { content } => plain_text_of(content),
        Block::List { items, .. } => joined_text(items.iter().map(|item| plain_text_of(&item.content))),
        Block::Table { caption, rows } => match caption.as_deref().map(str::trim).filter(|caption| !caption.is_empty()) {
            Some(caption) => caption.to_string(),
            None => joined_text(rows.iter().flat_map(|row| row.cells.iter()).map(|cell| plain_text_of(&cell.content))),
        },
        Block::Code { .. } | Block::Math { .. } | Block::Diagram { .. } | Block::Opaque { .. } => String::new(),
    }
}

fn plain_text_of(inlines: &[Inline]) -> String {
    inlines.iter().map(Inline::plain_text).collect::<String>().trim().to_string()
}

fn joined_text(parts: impl Iterator<Item = String>) -> String {
    parts.filter(|text| !text.is_empty()).collect::<Vec<_>>().join(" ")
}

fn invalid_params(spec: &TransformPassSpec, message: &str) -> (TransformPassReport, Vec<Diagnostic>) {
    let diagnostic = pass_diagnostic(
        "panduck.transform.invalid-params",
        DiagnosticSeverity::Warning,
        format!("Transform pass \"{}\" was skipped: {message}", spec.name),
    )
    .with_note(
        Message::new("panduck.transform.invalid-params")
            .with_arg("pass", MessageArg::Text(spec.name.clone()))
            .with_arg("message", MessageArg::Text(message.to_string())),
    );
    (
        TransformPassReport {
            name: spec.name.clone(),
            status: TransformPassStatus::InvalidParams,
            changed_nodes: 0,
            replaced_occurrences: None,
            swept_nodes: None,
            target_kinds: normalized_kinds(spec),
            description: spec.description.clone(),
            message: Some(message.to_string()),
        },
        vec![diagnostic],
    )
}

fn pass_diagnostic(code: &str, severity: DiagnosticSeverity, fallback: String) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code),
        severity,
        DiagnosticOrigin::new("panduck", "transform"),
        Message::new(code).with_fallback(fallback),
    )
}

fn traversal_of(spec: &TransformPassSpec) -> Traversal {
    match spec.traversal.as_deref() {
        Some("top-down") => Traversal::TopDown,
        _ => Traversal::BottomUp,
    }
}

fn normalized_kinds(spec: &TransformPassSpec) -> Option<Vec<String>> {
    if spec.target_kinds.is_empty() { None } else { Some(spec.target_kinds.clone()) }
}

/// Stable lowercase identifier for a block payload kind.
fn block_kind(block: &Block) -> &'static str {
    match block {
        Block::Section { .. } => "section",
        Block::Paragraph { .. } => "paragraph",
        Block::List { .. } => "list",
        Block::Table { .. } => "table",
        Block::Quote { .. } => "quote",
        Block::Code { .. } => "code",
        Block::Math { .. } => "math",
        Block::Diagram { .. } => "diagram",
        Block::Opaque { .. } => "opaque",
    }
}

fn descendants(graph: &DocumentGraph, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut stack = match graph.block(id) {
        Some(node) => child_ids(&node.block),
        None => Vec::new(),
    };
    while let Some(current) = stack.pop() {
        if out.contains(&current) {
            continue;
        }
        out.push(current);
        if let Some(node) = graph.block(current) {
            stack.extend(child_ids(&node.block));
        }
    }
    out
}

fn child_ids(block: &Block) -> Vec<NodeId> {
    match block {
        Block::Section { children, .. } => children.clone(),
        Block::List { items, .. } => items.iter().flat_map(|item| item.children.iter().copied()).collect(),
        _ => Vec::new(),
    }
}

/// Drop relation edges and provenance records that point at removed nodes.
///
/// `delete_block` intentionally does not rewrite semantic relations or
/// provenance, so without this step the subsequent Notedown export would fail
/// validation with dangling endpoints.
fn prune_dangling_references(graph: &mut DocumentGraph) {
    let node_ids: HashSet<NodeId> = graph.blocks.iter().map(|node| node.id).collect();
    let asset_ids: HashSet<AssetId> = graph.assets.iter().map(|asset| asset.id).collect();
    graph.relations.retain(|relation| {
        endpoint_exists(&relation.source, &node_ids, &asset_ids) && endpoint_exists(&relation.target, &node_ids, &asset_ids)
    });
    graph.sources.retain(|source| node_ids.contains(&source.node));
}

fn endpoint_exists(endpoint: &RelationEndpoint, node_ids: &HashSet<NodeId>, asset_ids: &HashSet<AssetId>) -> bool {
    match endpoint {
        RelationEndpoint::Node(id) => node_ids.contains(id),
        RelationEndpoint::Asset(id) => asset_ids.contains(id),
        RelationEndpoint::Document(_) | RelationEndpoint::ExternalUri(_) | RelationEndpoint::Unresolved { .. } => true,
    }
}

// ---------------------------------------------------------------------------
// Text replacement
// ---------------------------------------------------------------------------

fn rewrite_block(block: &Block, search: &str, replacement: &str) -> Option<(Block, usize)> {
    match block {
        Block::Section { level, title, children } => {
            let (title, occurrences) = rewrite_inlines(title, search, replacement);
            (occurrences > 0).then(|| (Block::Section { level: *level, title, children: children.clone() }, occurrences))
        }
        Block::Paragraph { content } => {
            let (content, occurrences) = rewrite_inlines(content, search, replacement);
            (occurrences > 0).then(|| (Block::Paragraph { content }, occurrences))
        }
        Block::Quote { content } => {
            let (content, occurrences) = rewrite_inlines(content, search, replacement);
            (occurrences > 0).then(|| (Block::Quote { content }, occurrences))
        }
        Block::List { ordered, items } => {
            let mut occurrences = 0;
            let items = items
                .iter()
                .map(|item| {
                    let (content, count) = rewrite_inlines(&item.content, search, replacement);
                    occurrences += count;
                    ListItem { content, children: item.children.clone(), number: item.number }
                })
                .collect::<Vec<_>>();
            (occurrences > 0).then(|| (Block::List { ordered: *ordered, items }, occurrences))
        }
        Block::Table { caption, rows } => {
            let mut occurrences = 0;
            let caption = caption.as_ref().map(|caption| {
                occurrences += caption.matches(search).count();
                caption.replace(search, replacement)
            });
            let rows = rows
                .iter()
                .map(|row| {
                    let cells = row
                        .cells
                        .iter()
                        .map(|cell| {
                            let (content, count) = rewrite_inlines(&cell.content, search, replacement);
                            occurrences += count;
                            notedown_ir::TableCell { content, colspan: cell.colspan, rowspan: cell.rowspan }
                        })
                        .collect::<Vec<_>>();
                    TableRow { is_header: row.is_header, cells }
                })
                .collect::<Vec<_>>();
            (occurrences > 0).then(|| (Block::Table { caption, rows }, occurrences))
        }
        // Diagram source is authored data, not inline prose, so text
        // replacement must not rewrite or corrupt it.
        Block::Code { .. } | Block::Math { .. } | Block::Diagram { .. } | Block::Opaque { .. } => None,
    }
}

fn rewrite_inlines(inlines: &[notedown_ir::Inline], search: &str, replacement: &str) -> (Vec<notedown_ir::Inline>, usize) {
    use notedown_ir::Inline;

    let mut occurrences = 0;
    let mut rewritten = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            Inline::Text { text } => {
                let count = text.matches(search).count();
                occurrences += count;
                rewritten.push(Inline::Text { text: text.replace(search, replacement) });
            }
            Inline::Styled { style, children } => {
                let (children, count) = rewrite_inlines(children, search, replacement);
                occurrences += count;
                rewritten.push(Inline::Styled { style: style.clone(), children });
            }
            Inline::Attributed { attributes, children } => {
                let (children, count) = rewrite_inlines(children, search, replacement);
                occurrences += count;
                rewritten.push(Inline::Attributed { attributes: attributes.clone(), children });
            }
            Inline::Link { target, display, title } => {
                let (display, count) = rewrite_inlines(display, search, replacement);
                occurrences += count;
                rewritten.push(Inline::Link { target: target.clone(), display, title: title.clone() });
            }
            Inline::Image { source, alt, title, asset } => {
                let (alt, count) = rewrite_inlines(alt, search, replacement);
                occurrences += count;
                rewritten.push(Inline::Image { source: source.clone(), alt, title: title.clone(), asset: *asset });
            }
            Inline::Note { label, display } => {
                let (display, count) = rewrite_optional_inlines(display.as_deref(), search, replacement);
                occurrences += count;
                rewritten.push(Inline::Note { label: label.clone(), display });
            }
            Inline::Citation { key, display } => {
                let (display, count) = rewrite_optional_inlines(display.as_deref(), search, replacement);
                occurrences += count;
                rewritten.push(Inline::Citation { key: key.clone(), display });
            }
            other => rewritten.push(other.clone()),
        }
    }
    (rewritten, occurrences)
}

fn rewrite_optional_inlines(
    inlines: Option<&[notedown_ir::Inline]>,
    search: &str,
    replacement: &str,
) -> (Option<Vec<notedown_ir::Inline>>, usize) {
    match inlines {
        Some(inlines) => {
            let (rewritten, occurrences) = rewrite_inlines(inlines, search, replacement);
            (Some(rewritten), occurrences)
        }
        None => (None, 0),
    }
}
