use notedown_ast::nodes::{StyleNode, TextSpan};
use pretty::RcDoc;

use super::*;

impl PrettyHTML for TextSpan {
    fn pretty_html(&self, cfg: &mut PrettyRenderer) -> RcDoc<()> {
        match self {
            TextSpan::HTMLRawInline(_) => match cfg.config.trust_raw_html {
                true => {
                    todo!()
                }
                false => RcDoc::as_string(""),
            },
            TextSpan::Normal(v) => RcDoc::as_string(v),
            TextSpan::Emoji(v) => RcDoc::as_string(v),
            TextSpan::Escaped(v) => RcDoc::as_string(v),
            TextSpan::SoftNewline => {
                unimplemented!()
            }
            TextSpan::HardNewline => match cfg.xhtml {
                true => RcDoc::as_string("<br/>"),
                false => RcDoc::as_string("<br>"),
            },
            TextSpan::CheckBox(_) => {
                unimplemented!()
            }
            TextSpan::Empty => RcDoc::as_string(""),
            TextSpan::Raw(_) => {
                unimplemented!()
            }
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
