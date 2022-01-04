use super::*;

impl IntoLaTeX for ListView {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            Self::Ordered(v) => v.into_latex(cfg, ctx),
            Self::Orderless(v) => v.into_latex(cfg, ctx),
        }
    }
}

impl IntoLaTeX for OrderedList {
    /// https://www.overleaf.com/learn/latex/Lists#The_enumerate_environment_for_numbered_.28ordered.29_lists
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let mut enumerate = text_ref("\\begin{enumerate}")
            .append(cfg.list_config.ordered_style.as_str())
            .append(hard_break(1));
        for item in &self.children {
            enumerate = enumerate //
                .append(space(4))
                .append(item.into_latex(cfg, ctx))
                .append(hard_break(1))
        }
        enumerate.append("\\end{enumerate}")
    }
}

impl IntoLaTeX for OrderlessList {
    /// https://www.overleaf.com/learn/latex/Lists#The_itemize_environment_for_bulleted_.28unordered.29_lists
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let mut itemize = text_ref("\\begin{itemize}").append(hard_break(1));
        ctx.is_orderless_env = true;
        for item in &self.children {
            itemize = itemize //
                .append(space(4))
                .append(item.into_latex(cfg, ctx))
                .append(hard_break(1))
        }
        ctx.is_orderless_env = false;
        itemize.append("\\end{itemize}")
    }
}

impl IntoLaTeX for ListItem {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let mut item = text_ref("\\item");
        if ctx.is_orderless_env {
            let style = text_ref(cfg.list_config.orderless_style.as_str());
            item = item.append(style)
        }
        item = item.append(space(1));
        for i in &self.rest {
            item = item.append(i.value.into_latex(cfg, ctx))
        }
        item
    }
}
