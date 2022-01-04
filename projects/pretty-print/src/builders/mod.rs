use std::{borrow::Cow, fmt::Display};

use pretty::{RcAllocator, RcDoc};

use crate::PrettyPrint;

pub use self::{open_closed_group::OpenClosedGroup, prefix_group::PrefixGroup, surround_group::SurroundGroup};

mod open_closed_group;
mod prefix_group;
mod surround_group;

/// nothing output
pub fn nil<'a>() -> PrettyPrint<'a> {
    RcDoc::nil()
}

/// a text ref, e.g. [`&str`], [`&String`]
pub fn text_ref<'a, S: Into<Cow<'a, str>>>(s: S) -> PrettyPrint<'a> {
    RcDoc::text(s)
}

/// a owned text, e.g. [`Cow::Owned`], [`String`]
pub fn text_own<'a, S: Display>(s: S) -> PrettyPrint<'a> {
    RcDoc::as_string(s)
}

/// break the line in any cases
pub fn hard_break<'a>(n: usize) -> PrettyPrint<'a> {
    RcDoc::text("\n".repeat(n))
}

/// add ` ` in any cases
pub fn space<'a>(n: usize) -> PrettyPrint<'a> {
    RcDoc::text(" ".repeat(n))
}

/// - `NIL` if inline
/// - `\n` if break
pub fn nil_or_newline<'a>() -> PrettyPrint<'a> {
    RcDoc::hardline().flat_alt(RcDoc::nil())
}

/// - `\s` if inline
/// - `\n` if break line
pub fn space_or_newline<'a>() -> PrettyPrint<'a> {
    RcDoc::hardline().flat_alt(RcDoc::nil())
}
