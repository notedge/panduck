use pretty::RcDoc;

use super::*;

impl PrettyHTML for ASTNode {
    fn pretty_html(&self, f: &mut PrettyRenderer) -> RcDoc<()> {
        match &self.value {
            ASTKind::Statements(v) => RcDoc::intersperse(v.iter().map(|x| x.pretty_html(f)), RcDoc::line()).nest(1).group(),
            ASTKind::Paragraph(v) => RcDoc::text("<p>")
                .append(RcDoc::intersperse(v.iter().map(|x| x.pretty_html(f)), RcDoc::line()).nest(1).group())
                .append(RcDoc::text("</p>")),
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
            ASTKind::TextSpan(v) => v.pretty_html(f),
            ASTKind::StyledSpan(v) => v.pretty_html(f),
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
