use std::borrow::Cow;

use notedown_ast::nodes::{StyleNode, TextSpan};

use super::*;

impl IntoHTML for TextSpan {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a> {
        match self {
            TextSpan::HTMLRawInline(_) => match cfg.trust_raw_html {
                true => {
                    todo!()
                }
                false => nil(),
            },
            TextSpan::Normal(v) => text(v),
            TextSpan::Emoji(v) => text(*v),
            TextSpan::Escaped(v) => text(v.to_string()),
            TextSpan::SoftNewline => {
                unimplemented!()
            }
            TextSpan::HardNewline => text(cfg.xhtml.tag("br")),
            TextSpan::CheckBox(_) => {
                unimplemented!()
            }
            TextSpan::Empty => text(""),
            TextSpan::Raw(_) => {
                unimplemented!()
            }
        }
    }
}
impl IntoHTML for StyleNode {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a> {
        let group = OpenClosedGroup {};
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => group.print("<em>", "</em>", self.children.iter().map(|x| x.into_html(cfg, ctx))),
            StyleKind::Strong => group.print("<strong>", "</strong>", self.children.iter().map(|x| x.into_html(cfg, ctx))),
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
