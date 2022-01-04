use super::*;

impl IntoLaTeX for ListView {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            Self::Ordered(v) => v.into_latex(cfg, ctx),
            Self::Orderless(v) => v.into_latex(cfg, ctx),
        }
    }
}

impl IntoLaTeX for OrderedList {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for OrderlessList {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}
