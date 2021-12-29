use notedown_ast::{command::Command, Value};

use super::*;

impl<G> IntoSycamore<G> for Command
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let a: G = GenericNode::element("cmd");
        a.update_inner_text(&format!("{:?}", self));
        return a;
    }
}

impl<G> IntoSycamore<G> for Value
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let a: G = GenericNode::element("value");
        a.update_inner_text(&format!("{:?}", self));
        return a;
    }
}
