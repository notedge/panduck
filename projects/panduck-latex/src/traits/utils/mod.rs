use pretty::RcDoc;

pub fn empty<'a>() -> RcDoc<'a> {
    RcDoc::nil()
}

pub fn text(s: &str) -> RcDoc {
    RcDoc::text(s)
}

/// skip one line
pub fn block_break<'a>() -> RcDoc<'a> {
    RcDoc::text("\n\n")
}

/// skip one line
pub fn new_line<'a>() -> RcDoc<'a> {
    RcDoc::text("\n")
}

/// - `NIL` if inline
/// - `\n` if break
pub fn empty_or_newline<'a>() -> RcDoc<'a> {
    RcDoc::hardline().flat_alt(RcDoc::nil())
}

/// - `\s` if inline
/// - `\n` if break line
pub fn space_or_newline<'a>() -> RcDoc<'a> {
    RcDoc::hardline().flat_alt(RcDoc::nil())
}
