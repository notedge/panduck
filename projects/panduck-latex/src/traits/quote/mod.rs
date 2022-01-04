use super::*;

impl IntoLaTeX for QuoteBlock {
    /// https://www.overleaf.com/learn/latex/Typesetting_quotations
    ///
    /// ```tex
    /// \begin{displayquote}
    /// the quoted texts
    /// \end{displayquote}
    /// ```
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let inner = PrettyPrint::intersperse(
            self.body.iter().map(|x| x.into_latex(cfg, ctx)),
            hard_break(2),
        )
        .group();
        text_ref("\\begin{displayquote}")
            .append(hard_break(1))
            .append(inner)
            .append(hard_break(1))
            .append("\\end{displayquote}")
    }
}
