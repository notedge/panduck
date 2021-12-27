use super::*;

impl IntoLaTeX for TextSpan {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
        let _ = (cfg, ctx);
        todo!()
    }
}

impl IntoLaTeX for StyleNode {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
        let _ = (cfg, ctx);
        todo!()
    }
}
