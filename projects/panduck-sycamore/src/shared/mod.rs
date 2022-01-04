use notedown_error::Result;
use sycamore::prelude::GenericNode;

use crate::{
    builder::{SycamoreBuilder, SycamoreConfig, SycamoreContext},
    traits::IntoSycamore,
};

pub fn error_inline<G: GenericNode>(msg: &str) -> G {
    let node: G = GenericNode::element("span");
    node.set_class_name("debug-error");
    node.update_inner_text(msg);
    node
}

// pub fn error_block<G: GenericNode>(msg: &str) -> G {
//     let node: G = GenericNode::element("p");
//     node.set_class_name("debug-error");
//     node.update_inner_text(msg);
//     return node;
// }

pub fn push_nodes<T, G>(node: &G, children: Vec<T>, cfg: &SycamoreConfig, ctx: &mut SycamoreContext)
where
    G: GenericNode,
    T: IntoSycamore<G>,
{
    for i in children {
        node.append_child(&i.into_sycamore(cfg, ctx))
    }
}

pub fn unwrap_inner<G: GenericNode>(node: G) -> G {
    match node.first_child() {
        Some(s) => s,
        #[cfg(debug_assertions)]
        None => panic!("Illegal HTML content"),
        #[cfg(not(debug_assertions))]
        None => GenericNode::marker(),
    }
}

pub fn phantom_node<G: GenericNode>(html: Result<String>) -> G {
    let g: G = GenericNode::element("Phantom");
    match html {
        Ok(o) => g.dangerously_set_inner_html(&o),
        #[cfg(debug_assertions)]
        Err(e) => return error_inline(e.to_string().as_str()),
        #[cfg(not(debug_assertions))]
        Err(_) => return GenericNode::marker(),
    };
    unwrap_inner(g)
}
