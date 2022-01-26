use dioxus::{
    core::{DioxusElement, NodeFactory, VNode},
    prelude::dioxus_elements::{h1, h2, h3, h4, h5, h6},
};

use super::*;

impl IntoDioxus for Header {
    fn into_dioxus(self, cx: Scope<DioxusBuilder>) -> Element {
        cx.render(LazyNodes::new(move |f| match self.level {
            1 => f.element(h1, &[], &[], &[], None),
            2 => f.element(h2, &[], &[], &[], None),
            3 => f.element(h3, &[], &[], &[], None),
            4 => f.element(h4, &[], &[], &[], None),
            5 => f.element(h5, &[], &[], &[], None),
            _ => f.element(h6, &[], &[], &[], None),
        }))
    }
}
