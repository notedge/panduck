use notedown_ast::nodes::{StyleNode, TextSpan};

use super::*;

impl IntoHTML for TextSpan {
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a> {
        match self {
            TextSpan::HTMLRawInline(_) => match cfg.trust_raw_html {
                true => {
                    todo!()
                }
                false => PrettyPrint::nil(),
            },
            TextSpan::Normal(v) => text(v),
            TextSpan::Emoji(v) => text(*v),
            TextSpan::Escaped(v) => text(v.to_string()),
            TextSpan::SoftNewline => {
                unimplemented!()
            }
            TextSpan::HardNewline => match cfg.xhtml {
                true => text("<br/>"),
                false => text("<br>"),
            },
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
        match self.kind {
            StyleKind::Plain => {
                unimplemented!()
            }
            StyleKind::Emphasis => text("<em>")
                .append(
                    PrettyPrint::intersperse(self.children.iter().map(|x| x.into_html(cfg, ctx)), PrettyPrint::line())
                        .nest(1)
                        .group(),
                )
                .append(text("</em>")),
            StyleKind::Strong => text("<strong>")
                .append(
                    PrettyPrint::intersperse(self.children.iter().map(|x| x.into_html(cfg, ctx)), PrettyPrint::line())
                        .nest(1)
                        .group(),
                )
                .append(text("</strong>")),
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
