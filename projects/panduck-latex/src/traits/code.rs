use super::*;

impl IntoLaTeX for CodeNode {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        text_ref("\\begin{lstlisting}[language=")
            .append(&self.language)
            .append("]")
            .append(&self.code)
            .append("\\end{lstlisting}")
    }
}
