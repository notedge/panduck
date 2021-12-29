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
pub struct OpenClosedGroup<'a> {
    ident: usize,
    inline: &'a str,
    newline: &'a str,
    inline_end_mark: bool,
    block_end_mark: bool,
}

impl Default for OpenClosedGroup<'static> {
    fn default() -> Self {
        Self { ident: 4, inline: ", ", newline: ",", inline_end_mark: true, block_end_mark: true }
    }
}

impl<'a> OpenClosedGroup<'a> {
    pub fn print<'i, T, I>(&self, start: T, end: T, items: I) -> PrettyPrint<'i>
    where
        T: Into<Cow<'i, str>>,
        I: IntoIterator,
        I::Item: Pretty<'i, RcAllocator, ()>, // life time of input items
    {
        let inline = RcDoc::as_string(&self.inline);
        let newline = RcDoc::as_string(&self.newline).append(RcDoc::hardline());
        let separator = newline.flat_alt(inline);
        let middle = RcDoc::intersperse(items, separator);
        let mut middle = nil_or_newline().append(middle);
        if self.inline_end_mark && self.block_end_mark {
            middle = middle.append(RcDoc::as_string(&self.newline));
        }
        else if self.block_end_mark {
            todo!()
        }
        else if self.inline_end_mark {
            todo!()
        }
        let middle = middle.nest(self.ident as isize).append(nil_or_newline()).group();
        text(start).append(middle).append(text(end))
    }
}
