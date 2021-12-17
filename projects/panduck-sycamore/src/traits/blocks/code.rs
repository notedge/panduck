use super::*;
use crate::shared::phantom_node;

impl<G> IntoSycamore<G> for CodeNode
where
    G: GenericNode,
{
    #[cfg(feature = "local")]
    fn into_sycamore(self, builder: &SycamoreBuilder) -> G {
        let html = builder.config.code_config.syntect_config.render_html(&self);
        phantom_node(html)
    }
}
