use notedown_ast::nodes::MathBackend;

use crate::shared::phantom_node;

use super::*;

impl<G> IntoSycamore<G> for MathNode
where
    G: GenericNode,
{
    #[cfg(feature = "server")]
    fn into_sycamore(self, builder: &SycamoreBuilder) -> G {
        match self.format {
            MathBackend::LaTeX => {
                let html = builder.config.math_config.katex_config.render_html(&self);
                phantom_node(html)
            }
            MathBackend::AsciiMath => {
                todo!()
            }
            MathBackend::MathML => {
                todo!()
            }
        }
    }
}
