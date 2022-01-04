use pretty::{Pretty, RcAllocator, RcDoc};

use crate::PrettyPrint;

pub fn tex_inline_macro<'a, D>(cmd: &'a str, inner: D) -> PrettyPrint<'a>
where
    D: Pretty<'a, RcAllocator, ()>,
{
    RcDoc::text("\\").append(cmd).append("{").append(inner).append(RcDoc::text("}"))
}
