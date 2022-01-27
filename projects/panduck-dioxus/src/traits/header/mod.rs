use super::*;

impl IntoDioxus for Header {
    fn into_dioxus(self, cx: Scope<DioxusBuilder>) -> Element {
        let level = self.level;
        let id = self.id.as_str();
        let children = self.children.into_iter().map(|e| e.into_dioxus(cx));
        cx.render(LazyNodes::new_some(move |f: NodeFactory| -> VNode {
            let attributes = f.bump().alloc([f.attr("id", format_args!("{}", id), None, false)]);
            let children = f.bump().alloc([f.fragment_from_iter(children)]);
            match level {
                1 => f.element(dioxus_elements::h1, &[], attributes, children, None),
                2 => f.element(dioxus_elements::h2, &[], attributes, children, None),
                3 => f.element(dioxus_elements::h3, &[], attributes, children, None),
                4 => f.element(dioxus_elements::h4, &[], attributes, children, None),
                5 => f.element(dioxus_elements::h5, &[], attributes, children, None),
                _ => f.element(dioxus_elements::h6, &[], attributes, children, None),
            }
        }))
    }
}
