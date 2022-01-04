use super::*;

impl IntoLaTeX for Header {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        // Assuming the level setter is used, input is legal
        let mut header = match self.level {
            1 => text_ref("\\chapter").append("{"),
            2 => text_ref("\\section").append("{"),
            3 => text_ref("\\subsection").append("{"),
            4 => text_ref("\\subsubsection").append("{"),
            5 => text_ref("\\paragraph").append("{"),
            _ => text_ref("\\subparagraph").append("{"),
        };

        let inner = PrettyPrint::intersperse(
            self.children.iter().map(|x| x.into_latex(cfg, ctx)),
            nil_or_newline(),
        )
        .nest(1)
        .group();
        header = header.append(inner).append("}");
        header = header.append(nil_or_newline());
        match &self.id {
            None => {}
            Some(id) => {
                header = header.append("\\label{").append(id).append("}");
            }
        }
        return header;
    }
}
