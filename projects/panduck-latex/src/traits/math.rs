use super::*;

impl IntoLaTeX for MathNode {
    fn into_latex<'a>(&'a self, _: &LaTeXConfig, _: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self.format {
            MathBackend::LaTeX => render_latex(&self.kind, &self.raw),
            MathBackend::AsciiMath => {
                todo!()
            }
            MathBackend::MathML => {
                todo!()
            }
        }
    }
}

fn render_latex<'a>(kind: &MathKind, tex: &'a str) -> PrettyPrint<'a> {
    match kind {
        MathKind::Inline => {
            text_ref("$") //
                .append(tex)
                .append("$")
        }
        MathKind::Display => {
            text_ref("$") //
                .append("\\displaystyle")
                .append("{")
                .append(tex)
                .append("}")
                .append("$")
        }
        MathKind::BlockInline => {
            text_ref("$$") //
                .append("\\textstyle")
                .append("{")
                .append(tex)
                .append("}")
                .append("$$")
        }
        MathKind::BlockDisplay => {
            text_ref("$$") //
                .append(tex)
                .append("$$")
        }
    }
}
