use crate::shared::phantom_node;

use super::*;

impl<G> IntoSycamore<G> for MathNode
where
    G: GenericNode,
{
    #[cfg(feature = "local")]
    fn into_sycamore(self, builder: &SycamoreBuilder) -> G {
        match self.format.to_ascii_lowercase().as_str() {
            "tex" | "latex" => {
                let html = builder.config.math_config.katex_config.render_html(&self);
                phantom_node(html)
            }
            #[cfg(debug_assertions)]
            _ => return error_inline(&format!("Unknown math renderer {}", self.get_format())),
            #[cfg(not(debug_assertions))]
            _ => return GenericNode::marker(),
        }
    }
}
