use std::{
    borrow::{Borrow, BorrowMut, Cow},
    lazy::SyncLazy,
};

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

/// text
pub fn text<'a, S: Into<Cow<'a, str>>>(s: S) -> PrettyPrint<'a> {
    RcDoc::text(s)
}

/// skip one line
pub fn block_break<'a>() -> PrettyPrint<'a> {
    RcDoc::text("\n\n")
}

/// skip one line
pub fn newline<'a>() -> PrettyPrint<'a> {
    RcDoc::text("\n")
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
