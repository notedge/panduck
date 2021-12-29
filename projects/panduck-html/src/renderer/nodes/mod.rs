use super::*;

impl IntoHTML for ASTNode {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a> {
        match &self.value {
            ASTKind::Statements(v) => {
                PrettyPrint::intersperse(v.iter().map(|x| x.into_html(cfg, ctx)), nil_or_newline()).nest(1).group()
            }
            ASTKind::Paragraph(v) => PrettyPrint::text("<p>")
                .append(PrettyPrint::intersperse(v.iter().map(|x| x.into_html(cfg, ctx)), nil_or_newline()).nest(1).group())
                .append(text("</p>")),
            ASTKind::Delimiter(_) => {
                unimplemented!()
            }
            ASTKind::Header(_) => {
                unimplemented!()
            }
            ASTKind::TableView(_) => {
                unimplemented!()
            }
            ASTKind::ListView(_) => {
                unimplemented!()
            }
            ASTKind::CodeNode(_) => {
                unimplemented!()
            }
            ASTKind::MathNode(_) => {
                unimplemented!()
            }
            ASTKind::LinkNode(_) => {
                unimplemented!()
            }
            ASTKind::TextSpan(v) => v.into_html(cfg, ctx),
            ASTKind::StyledSpan(v) => v.into_html(cfg, ctx),
            ASTKind::Command(_) => {
                unimplemented!()
            }
            ASTKind::Value(_) => {
                unimplemented!()
            }
            ASTKind::QuoteNode(_) => {
                unimplemented!()
            }
        }
    }
}
