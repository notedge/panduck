use crate::shared::phantom_node;

use super::*;

impl<G> IntoSycamore<G> for TextSpan
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        match self {
            Self::Normal(s) => GenericNode::text_node(&s),
            Self::HTMLRawInline(s) => match cfg.trust_raw_html {
                true => phantom_node(Ok(s)),
                false => GenericNode::marker(),
            },
            Self::Escaped(c) => GenericNode::text_node(c.to_string().as_str()),
            Self::Emoji(c) => GenericNode::text_node(c.to_string().as_str()),
            Self::SoftNewline => GenericNode::text_node("\n"),
            Self::HardNewline => GenericNode::element("br"),
            Self::CheckBox(_) => {
                unimplemented!()
            }
            Self::Empty => GenericNode::marker(),
            Self::Raw(_) => {
                unimplemented!()
            }
        }
    }
}

impl<G> IntoSycamore<G> for StyleNode
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let node: G = self.kind.into_sycamore(cfg, ctx);
        // for i in self.children {
        //     node.append_child(&i.value.into_sycamore())
        // }
        push_nodes(&node, self.children, cfg, ctx);
        return node;
    }
}

impl<G> IntoSycamore<G> for StyleKind
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreConfig, _: &mut SycamoreContext) -> G {
        match self {
            Self::Plain => GenericNode::element("span"),
            Self::Emphasis => GenericNode::element("em"),
            Self::Underline => GenericNode::element("u"),
            Self::Undercover => GenericNode::element("u"),
            Self::Strong => GenericNode::element("strong"),
            Self::Delete => GenericNode::element("del"),
            Self::Insert => GenericNode::element("ins"),
            Self::ItalicBold => {
                unreachable!()
            }
            Self::Marking => {
                unimplemented!()
            }
            Self::Color(_, _, _, _) => {
                unimplemented!()
            }
            Self::Subscript => GenericNode::element("sub"),
            Self::Superscript => GenericNode::element("sup"),
        }
    }
}
