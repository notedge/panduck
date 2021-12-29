use std::borrow::Cow;

use pretty::{Pretty, RcAllocator, RcDoc};

use crate::PrettyPrint;

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

/// ```js
/// [1, 2, 3]
///
/// <
///  str
///  end
/// />
///
/// ```
///
///
pub struct OpenClosedGroup {
    ident: usize,
}

impl Default for OpenClosedGroup {
    fn default() -> Self {
        Self { ident: 4 }
    }
}

impl OpenClosedGroup {
    pub fn print<'a, S, I>(&self, start: S, end: S, items: I) -> PrettyPrint<'a>
    where
        S: Into<Cow<'a, str>>,
        I: IntoIterator,
        I::Item: Pretty<'a, RcAllocator, ()>,
    {
        let middle = PrettyPrint::intersperse(items, PrettyPrint::line());
        let middle = nil_or_newline().append(middle).nest(self.ident as isize).group();
        text(start).append(middle).append(nil_or_newline()).append(text(end))
    }
}
