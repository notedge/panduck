use pretty::{Pretty, RcAllocator};

use super::*;

impl IntoLaTeX for TextSpan {
    fn into_latex<'a>(&'a self, _: &LaTeXConfig, _: &mut LaTeXContext) -> RcDoc<'a, ()> {
        match self {
            TextSpan::Empty => RcDoc::text(""),
            TextSpan::Normal(_) => {
                unimplemented!()
            }
            TextSpan::Raw(_) => {
                unimplemented!()
            }
            TextSpan::HTMLRawInline(_) => {
                unimplemented!()
            }
            TextSpan::Emoji(_) => {
                unimplemented!()
            }
            TextSpan::Escaped(_) => {
                unimplemented!()
            }
            TextSpan::SoftNewline => {
                unimplemented!()
            }
            TextSpan::HardNewline => {
                unimplemented!()
            }
            TextSpan::CheckBox(_) => {
                unimplemented!()
            }
        }
    }
}

impl IntoLaTeX for StyleNode {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<'a, ()> {
        let inner =
            RcDoc::intersperse(self.children.iter().map(|x| x.into_latex(cfg, ctx)), RcDoc::softline_()).nest(1).group();
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => inline_style("emph", inner),
            StyleKind::Strong => inline_style("textbf", inner),
            StyleKind::ItalicBold => {
                unimplemented!()
            }
            StyleKind::Underline => inline_style("underline", inner),
            StyleKind::Undercover => {
                unimplemented!()
            }
            StyleKind::Marking => {
                unimplemented!()
            }
            StyleKind::Color(_, _, _, _) => {
                unimplemented!()
            }
            StyleKind::Delete => {
                unimplemented!()
            }
            StyleKind::Insert => {
                unimplemented!()
            }
            StyleKind::Subscript => {
                unimplemented!()
            }
            StyleKind::Superscript => {
                unimplemented!()
            }
        }
    }
}

fn inline_style<'a, A, D>(cmd: &str, inner: D) -> RcDoc<'a, A>
where
    D: Pretty<'a, RcAllocator, A>,
{
    RcDoc::text(format!("\\{}", cmd)).append("{").append(inner).append(RcDoc::text("}"))
}
