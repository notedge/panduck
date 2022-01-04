#![doc = include_str!("../Readme.md")]

pub use builders::*;
pub use tex::*;

mod builders;
mod tex;

/// Pretty print
pub type PrettyPrint<'a> = pretty::RcDoc<'a, ()>;
