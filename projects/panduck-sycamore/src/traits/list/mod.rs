use notedown_ast::nodes::{DetailedList, ListItem, OrderedList, OrderlessList, QuoteBlock};

use super::*;

impl<G> IntoSycamore<G> for ListView
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        match self {
            Self::Ordered(v) => v.into_sycamore(ctx),
            Self::Orderless(v) => v.into_sycamore(ctx),
        }
    }
}

impl<G> IntoSycamore<G> for OrderedList
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        let item: G = GenericNode::element("ol");
        push_nodes(&item, self.children, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for OrderlessList
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        let item: G = GenericNode::element("ul");
        push_nodes(&item, self.children, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for ListItem
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        let item: G = GenericNode::element("li");
        push_nodes(&item, self.rest, ctx);
        return item;
    }
}

impl<G> IntoSycamore<G> for QuoteBlock
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreBuilder) -> G {
        let item: G = GenericNode::element("blockquote");
        push_nodes(&item, self.body, cfg);
        return item;
    }
}

impl<G> IntoSycamore<G> for DetailedList
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        todo!()
    }
}
