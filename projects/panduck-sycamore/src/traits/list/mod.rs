use notedown_ast::nodes::{DetailedList, ListItem, OrderedList, OrderlessList, QuoteBlock};

use super::*;

impl<G> IntoSycamore<G> for ListView
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        match self {
            Self::Ordered(v) => v.into_sycamore(cfg, ctx),
            Self::Orderless(v) => v.into_sycamore(cfg, ctx),
        }
    }
}

impl<G> IntoSycamore<G> for OrderedList
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let item: G = GenericNode::element_from_tag("ol");
        push_nodes(&item, self.children, cfg, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for OrderlessList
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let item: G = GenericNode::element_from_tag("ul");
        push_nodes(&item, self.children, cfg, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for ListItem
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let item: G = GenericNode::element_from_tag("li");
        push_nodes(&item, self.rest, cfg, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for QuoteBlock
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let item: G = GenericNode::element_from_tag("blockquote");
        push_nodes(&item, self.body, cfg, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for DetailedList
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        todo!()
    }
}
