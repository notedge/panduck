use super::*;
use crate::shared::phantom_node;

mod code;
mod math;

impl<G> IntoSycamore<G> for Delimiter
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreBuilder) -> G {
        match self {
            Self::HorizontalRule => GenericNode::element("hr"),
            Self::HTMLRawBlock(s) => match cfg.config.trust_raw_html {
                true => phantom_node(Ok(s)),
                false => GenericNode::marker(),
            },
        }
    }
}

impl<G> IntoSycamore<G> for Header
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        let node = match self.level {
            1 => GenericNode::element("h1"),
            2 => GenericNode::element("h2"),
            3 => GenericNode::element("h3"),
            4 => GenericNode::element("h4"),
            5 => GenericNode::element("h5"),
            _ => GenericNode::element("h6"),
        };
        push_nodes(&node, self.children, ctx);
        return node;
    }
}
