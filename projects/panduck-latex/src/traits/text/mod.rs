use super::*;

impl IntoLaTeX for TextSpan {
    fn into_latex(self, _: &LaTeXConfig, _: &mut LaTeXContext) -> RcDoc<()> {
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
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => RcDoc::text("\\emph{")
                .append(
                    RcDoc::intersperse(self.children.into_iter().map(|x| x.into_latex(cfg, ctx)), RcDoc::softline_())
                        .nest(1)
                        .group(),
                )
                .append(RcDoc::text("}")),
            StyleKind::Strong => RcDoc::text("\\textbf{")
                .append(
                    RcDoc::intersperse(self.children.into_iter().map(|x| x.into_latex(cfg, ctx)), RcDoc::softline_())
                        .nest(1)
                        .group(),
                )
                .append(RcDoc::text("}")),
            StyleKind::ItalicBold => {
                unimplemented!()
            }
            StyleKind::Underline => RcDoc::text("\\underline{")
                .append(
                    RcDoc::intersperse(self.children.into_iter().map(|x| x.into_latex(cfg, ctx)), RcDoc::softline_())
                        .nest(1)
                        .group(),
                )
                .append(RcDoc::text("}")),
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
