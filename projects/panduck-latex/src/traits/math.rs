use super::*;

impl IntoLaTeX for MathNode {
    fn into_latex<'a>(&'a self, _: &LaTeXConfig, _: &mut LaTeXContext) -> RcDoc<'a, ()> {
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

fn render_latex<'a>(kind: &MathKind, tex: &'a str) -> RcDoc<'a, ()> {
    match kind {
        MathKind::Inline => {
            RcDoc::text("$") //
                .append(tex)
                .append("$")
        }
        MathKind::Display => {
            RcDoc::text("$") //
                .append("\\displaystyle")
                .append("{")
                .append(tex)
                .append("}")
                .append("$")
        }
        MathKind::BlockInline => {
            RcDoc::text("$$") //
                .append("\\textstyle")
                .append("{")
                .append(tex)
                .append("}")
                .append("$$")
        }
        MathKind::BlockDisplay => {
            RcDoc::text("$$") //
                .append(tex)
                .append("$$")
        }
    }
}
