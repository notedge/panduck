#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../readme.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]

pub mod ast;
pub mod reader;
pub mod writer;

/// Oak Markdown lexer/parser surface for this adapter.
pub mod oak {
    pub use oak_markdown::{
        MarkdownLanguage, MarkdownLexer, MarkdownParser, MarkdownRoot, MarkdownTokenType,
    };
}

/// Notedown document IR produced by Panduck readers.
pub mod ir {
    pub use notedown_ir::{
        Block, DocumentGraph, DocumentMetadata, IdAllocator, Inline, LossMarker, SemanticStatus,
    };
}
