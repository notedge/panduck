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
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let mut itemize = text_ref("\\begin{itemize}").append(hard_break(1));
        for item in &self.children {
            itemize = itemize //
                .append(space(4))
                .append(item.into_latex(cfg, ctx))
                .append(hard_break(1))
        }
        itemize.append("\\end{itemize}")
    }
}

impl IntoLaTeX for ListItem {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        let mut item = text_ref("\\item").append(space(1));
        for i in &self.rest {
            item = item.append(i.value.into_latex(cfg, ctx))
        }
        item
    }
}
