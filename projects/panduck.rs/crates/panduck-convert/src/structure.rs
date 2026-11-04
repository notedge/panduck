//! Built-in structural nesting guard for text formats.
//!
//! `recursion_depth` was removed from the CLI budget contract, so the pipeline
//! refuses pathological nesting at a fixed, non-configurable depth. The scan
//! runs on raw bytes before the format layer sees them: a document that would
//! drive Oak's tree builder deep enough to overflow the stack never reaches it.

use notedown_formats::{FormatError, from_format_error};
use panduck_types::{AdapterError, Result};

use crate::format_error;

/// Maximum structural nesting depth accepted for text formats.
///
/// This is a hard-coded guard, not a configurable budget. HTML element depth,
/// Markdown blockquote runs, and Markdown bracket/parenthesis nesting are all
/// measured against it.
pub const MAX_STRUCTURE_DEPTH: usize = 96;

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr",
];

/// Rejects text inputs whose structural nesting exceeds [`MAX_STRUCTURE_DEPTH`].
///
/// Formats without a structural parser (containers, PDF, DOC) return early and
/// keep relying on their own decode limits. Exceeding the limit is reported as
/// `notedown.format.budget-exhausted`, which the CLI maps to exit code 7 without
/// publishing any output.
pub(crate) fn enforce(from: &str, label: &str, bytes: &[u8]) -> Result<()> {
    let depth = match from {
        "html" => html_depth(bytes),
        "markdown" | "notedown" => markdown_depth(bytes),
        _ => return Ok(()),
    };
    if depth <= MAX_STRUCTURE_DEPTH {
        return Ok(());
    }
    let _ = label;
    format_error::fail(AdapterError::diagnostic(from_format_error(&FormatError::budget(
        from,
        format!("structure nesting depth {depth} exceeds the built-in limit {MAX_STRUCTURE_DEPTH}"),
    ))))
}

/// Peak element depth in an HTML byte stream.
///
/// Comments, doctype/processing instructions, and raw text inside
/// `script`/`style` are skipped so their contents cannot inflate the count.
fn html_depth(bytes: &[u8]) -> usize {
    let mut index = 0;
    let mut depth = 0usize;
    let mut peak = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"<!--") {
            index = find_sequence(bytes, index + 4, b"-->").map_or(bytes.len(), |end| end + 3);
            continue;
        }
        let Some(next) = bytes.get(index + 1).copied() else { break };
        if next == b'!' || next == b'?' {
            index = find_byte(bytes, index + 2, b'>').map_or(bytes.len(), |end| end + 1);
            continue;
        }
        if next == b'/' {
            depth = depth.saturating_sub(1);
            index = find_byte(bytes, index + 2, b'>').map_or(bytes.len(), |end| end + 1);
            continue;
        }
        if !next.is_ascii_alphabetic() {
            index += 1;
            continue;
        }
        let name_start = index + 1;
        let mut name_end = name_start;
        while name_end < bytes.len() {
            let byte = bytes[name_end];
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':' | b'_') {
                name_end += 1;
            }
            else {
                break;
            }
        }
        let Some(tag_end) = find_byte(bytes, name_end, b'>') else { break };
        let name = &bytes[name_start..name_end];
        let self_closing = bytes[tag_end - 1] == b'/';
        let void = VOID_ELEMENTS.iter().any(|element| element.as_bytes().eq_ignore_ascii_case(name));
        if !self_closing && !void {
            depth += 1;
            peak = peak.max(depth);
        }
        index = tag_end + 1;
        if !self_closing && (name.eq_ignore_ascii_case(b"script") || name.eq_ignore_ascii_case(b"style")) {
            let mut closing = Vec::with_capacity(name.len() + 2);
            closing.extend_from_slice(b"</");
            closing.extend_from_slice(name);
            index = find_sequence_ignore_ascii_case(bytes, index, &closing).unwrap_or(bytes.len());
        }
    }
    peak
}

/// Peak nesting depth in a Markdown byte stream.
///
/// Fenced code is skipped entirely, so chart specs and other embedded bracket
/// payloads do not count. Outside fences, the scan tracks blockquote markers,
/// square-bracket nesting, and parenthesis nesting; inline backtick spans are
/// skipped so code samples stay inert.
fn markdown_depth(bytes: &[u8]) -> usize {
    let mut peak = 0usize;
    let mut bracket = 0usize;
    let mut parenthesis = 0usize;
    let mut in_fence = false;
    let mut in_inline_code = false;
    for line in bytes.split(|byte| *byte == b'\n') {
        let trimmed = skip_spaces(line);
        if trimmed.starts_with(b"```") || trimmed.starts_with(b"~~~") {
            in_fence = !in_fence;
            in_inline_code = false;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut quote = 0usize;
        let mut cursor = 0usize;
        while cursor < line.len() && (line[cursor] == b' ' || line[cursor] == b'>') {
            if line[cursor] == b'>' {
                quote += 1;
            }
            cursor += 1;
        }
        peak = peak.max(quote);
        for byte in &line[cursor..] {
            if *byte == b'`' {
                in_inline_code = !in_inline_code;
                continue;
            }
            if in_inline_code {
                continue;
            }
            match byte {
                b'[' => {
                    bracket += 1;
                    peak = peak.max(quote + bracket);
                }
                b']' => bracket = bracket.saturating_sub(1),
                b'(' => {
                    parenthesis += 1;
                    peak = peak.max(quote + parenthesis);
                }
                b')' => parenthesis = parenthesis.saturating_sub(1),
                _ => {}
            }
        }
    }
    peak
}

fn skip_spaces(line: &[u8]) -> &[u8] {
    let mut offset = 0;
    while offset < line.len() && line[offset] == b' ' {
        offset += 1;
    }
    &line[offset..]
}

fn find_byte(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    bytes.get(from..)?.iter().position(|byte| *byte == needle).map(|offset| from + offset)
}

fn find_sequence(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes.get(from..)?.windows(needle.len()).position(|window| window == needle).map(|offset| from + offset)
}

fn find_sequence_ignore_ascii_case(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|offset| from + offset)
}
