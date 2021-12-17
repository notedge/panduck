use super::*;
use crate::shared::phantom_node;

impl<G> IntoSycamore<G> for TextNode
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreBuilder) -> G
    where
        G: GenericNode,
    {
        match self {
            Self::Normal(s) => GenericNode::text_node(&s),
            Self::HTMLRawInline(s) => match cfg.config.trust_raw_html {
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
        }
    }
}

impl<G> IntoSycamore<G> for StyleNode
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        let node: G = self.kind.into_sycamore(ctx);
        // for i in self.children {
        //     node.append_child(&i.value.into_sycamore())
        // }
        push_nodes(&node, self.children, ctx);
        return node;
    }
}

impl<G> IntoSycamore<G> for StyleKind
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        match self {
            Self::Plain => GenericNode::element("span"),
            Self::Emphasis => GenericNode::element("em"),
            Self::Underline => GenericNode::element("u"),
            Self::Undercover => GenericNode::element("u"),
            Self::Strong => GenericNode::element("strong"),
            Self::Delete => GenericNode::element("del"),
            Self::Insert => GenericNode::element("ins"),
            Self::ItalicBold => {
                unimplemented!()
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
