#![feature(try_trait_v2)]
#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../readme.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]

//! # Panduck Core
//! 
//! `panduck-types` is the core type definition library within the Panduck project, providing a unified type system, error handling, and serialization capabilities required for cross-platform assemblers.
//! 
//! Refer to the [README.md](../readme.md) for more information.

mod errors;
pub mod generator;
pub mod helpers;
pub mod lexer;
pub mod parser;
pub mod reader;
pub mod writer;

pub use crate::{
    errors::{PanduckErrorKind, PanduckDiagnostics, PanduckError, Result},
    generator::TextWriter,
    reader::BinaryReader,
    writer::BinaryWriter,
};
