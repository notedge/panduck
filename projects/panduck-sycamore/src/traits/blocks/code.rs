use crate::shared::phantom_node;

use super::*;

impl<G> IntoSycamore<G> for CodeNode
where
    G: GenericNode,
{
    #[cfg(feature = "server")]
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let html = cfg.code_config.syntect_config.render_html(&self);
        phantom_node(html)
    }
}
