use super::*;
use notedown_ast::nodes::StyleNode;
use pretty::RcDoc;

impl PrettyHTML for TextNode {
    fn pretty_html(&self, cfg: &mut PrettyRenderer) -> RcDoc<()> {
        match self {
            TextNode::HTMLRawInline(_) => match cfg.config.trust_raw_html {
                true => {
                    todo!()
                }
                false => RcDoc::as_string(""),
            },
            TextNode::Normal(v) => RcDoc::as_string(v),
            TextNode::Emoji(v) => RcDoc::as_string(v),
            TextNode::Escaped(v) => RcDoc::as_string(v),
            TextNode::SoftNewline => {
                unimplemented!()
            }
            TextNode::HardNewline => match cfg.xhtml {
                true => RcDoc::as_string("<br/>"),
                false => RcDoc::as_string("<br>"),
            },
            TextNode::CheckBox(_) => {
                unimplemented!()
            }
            TextNode::Empty => RcDoc::as_string(""),
        }
    }
}
impl PrettyHTML for StyleNode {
    fn pretty_html(&self, f: &mut PrettyRenderer) -> RcDoc<()> {
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => RcDoc::text("<em>")
                .append(RcDoc::intersperse(self.children.iter().map(|x| x.pretty_html(f)), RcDoc::line()).nest(1).group())
                .append(RcDoc::text("</em>")),
            StyleKind::Strong => RcDoc::text("<strong>")
                .append(RcDoc::intersperse(self.children.iter().map(|x| x.pretty_html(f)), RcDoc::line()).nest(1).group())
                .append(RcDoc::text("</strong>")),
            StyleKind::ItalicBold => {
                unimplemented!()
            }
            StyleKind::Underline => {
                unimplemented!()
            }
            StyleKind::Undercover => {
                unimplemented!()
            }
            StyleKind::Marking => {
                unimplemented!()
            }
            StyleKind::Color(_, _, _, _) => {
                unimplemented!()
            }
            StyleKind::Delete => {
                unimplemented!()
            }
            StyleKind::Insert => {
                unimplemented!()
            }
            StyleKind::Subscript => {
                unimplemented!()
            }
            StyleKind::Superscript => {
                unimplemented!()
            }
        }
    }
}
