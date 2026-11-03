//! Block-local spans promoted to authored document spans.
//!
//! Chart and diagram frontends report byte ranges relative to the authored
//! block text. Hosts add the block start so a loss marker points at the exact
//! document bytes instead of an offset that only makes sense inside the fence.

use std::ops::Range;

use notedown_ir::SourceSpan;

pub(crate) fn absolute_span(block: Option<SourceSpan>, local: Option<Range<usize>>) -> Option<SourceSpan> {
    match (block, local) {
        (Some(block), Some(local)) => SourceSpan::new(block.start as usize + local.start, block.start as usize + local.end),
        _ => block,
    }
}
