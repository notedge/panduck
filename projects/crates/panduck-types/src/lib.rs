#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../readme.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]

//! Panduck conversion contracts and binary helpers.
//!
//! Document semantics live in [`notedown_ir::DocumentGraph`] (`notedown-ir`), not in this crate.
//! Adapter failures use [`AdapterError`]. Syntax and container diagnostics stay in Oak and Acorn.

mod errors;
pub mod generator;
pub mod helpers;
pub mod lexer;
pub mod parser;
pub mod reader;
pub mod writer;

pub use crate::{
    errors::{AdapterError, Result},
    generator::TextWriter,
    reader::BinaryReader,
    writer::BinaryWriter,
};

/// Notedown document semantic IR — Panduck reader/writer hub type.
pub use notedown_ir;
