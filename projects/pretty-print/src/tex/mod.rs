use pretty::{Pretty, RcAllocator, RcDoc};

use crate::PrettyPrint;

pub fn tex_inline_text<'a, D>(cmd: D, inner: D) -> PrettyPrint<'a>
where
    D: Pretty<'a, RcAllocator, ()>,
{
    RcDoc::text("\\").append(cmd).append("{").append(inner).append(RcDoc::text("}"))
}
