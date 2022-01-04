use super::*;

impl IntoLaTeX for TextSpan {
    fn into_latex<'a>(&'a self, _: &LaTeXConfig, _: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            TextSpan::Empty => nil(),
            TextSpan::Normal(s) => text_ref(s),
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
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let inner = PrettyPrint::intersperse(
            self.children.iter().map(|x| x.into_latex(cfg, ctx)),
            block_break(),
        )
        .nest(1)
        .group();
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => tex_inline_macro("emph", inner),
            StyleKind::Strong => tex_inline_macro("textbf", inner),
            StyleKind::ItalicBold => {
                unimplemented!()
            }
            StyleKind::Underline => tex_inline_macro("underline", inner),
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
