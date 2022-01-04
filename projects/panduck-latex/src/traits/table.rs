use super::*;

impl IntoLaTeX for TableView {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            Self::SimpleTable(v) => v.into_latex(cfg, ctx),
        }
    }
}

impl IntoLaTeX for SimpleTable {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}
