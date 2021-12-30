use notedown_ast::nodes::{StyleNode, TextSpan};

use super::*;

impl IntoHTML for TextSpan {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, _: &mut HTMLContext) -> PrettyPrint<'a> {
        match self {
            Self::Empty => nil(),
            Self::HTMLRawInline(_) => match cfg.trust_raw_html {
                true => {
                    todo!()
                }
                false => nil(),
            },
            Self::Raw(_) => {
                unimplemented!()
            }
            Self::Normal(v) => text(v),
            Self::Emoji(v) => text(*v),
            Self::Escaped(v) => text(v.to_string()),
            Self::SoftNewline => {
                unimplemented!()
            }
            Self::HardNewline => text(cfg.xhtml.tag("br")),
            Self::CheckBox(_) => {
                unimplemented!()
            }
        }
    }
}
impl IntoHTML for StyleNode {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a> {
        let mut group = OpenClosedGroup::default();
        group.set_indent(cfg.indent).set_mark("", "");
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => group.build("<em>", "</em>", self.children.iter().map(|x| x.into_html(cfg, ctx))),
            StyleKind::Strong => group.build("<strong>", "</strong>", self.children.iter().map(|x| x.into_html(cfg, ctx))),
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
