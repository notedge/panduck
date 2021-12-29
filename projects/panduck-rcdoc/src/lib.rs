#![doc = include_str!("../Readme.md")]

use pretty::RcDoc;

pub use builders::*;

mod builders;

/// Pretty print
pub type PrettyPrint<'a> = RcDoc<'a, ()>;
