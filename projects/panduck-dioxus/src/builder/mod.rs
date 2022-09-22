use dioxus::prelude::LazyNodes;
use notedown_ast::ASTNode;

use crate::traits::IntoDioxus;

pub use self::{config::DioxusConfig, context::DioxusContext};

mod config;
mod context;

#[derive(Default)]
pub struct DioxusBuilder {
    config: DioxusConfig,
    context: DioxusContext,
}

impl DioxusBuilder {
    /// the html fragment
    pub fn render(&mut self, ast: ASTNode) -> String {
        todo!()
        // let node = ast.into_sycamore(&self.config, &mut self.context);
        // let view = View::<SsrNode>::new_node(node);
        // render_to_string(|| view)
    }
    /// a complete html
    pub fn render_standalone(&mut self, ast: ASTNode) -> String {
        todo!()
        // let node = ast.into_sycamore(&self.config, &mut self.context);
        // let html = SsrNode::element("html");
        // html.append_child(&self.html_head());
        // html.append_child(&node);
        // render_to_string(|| View::new_node(html))
    }

    fn html_head(&self) -> LazyNodes {
        todo!()
        // let head: G = GenericNode::element_from_tag("head");
        // head.append_child(&{
        //     let meta: G = GenericNode::element_from_tag("meta");
        //     meta.set_attribute("charset", "UTF-8");
        //     meta
        // });
        // head.append_child(&{
        //     let meta: G = GenericNode::element_from_tag("meta");
        //     meta.set_attribute("name", "viewport-8");
        //     meta.set_attribute("content", "'width=device-width, initial-scale=1.0'");
        //     meta
        // });
        // head.append_child(&{
        //     let meta: G = GenericNode::element_from_tag("link");
        //     meta.set_attribute("rel", "stylesheet");
        //     meta.set_attribute("href", "https://latex.now.sh/style.css");
        //     meta
        // });
        // return head;
    }
}
