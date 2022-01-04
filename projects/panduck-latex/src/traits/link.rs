use super::*;

impl IntoLaTeX for SmartLink {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            Self::ExternalResource(v) => v.into_latex(cfg, ctx),
            Self::EMail(v) => v.into_latex(cfg, ctx),
            Self::Normal(v) => v.into_latex(cfg, ctx),
            Self::Image(v) => v.into_latex(cfg, ctx),
            Self::Reference(v) => v.into_latex(cfg, ctx),
            Self::TwoWay(v) => v.into_latex(cfg, ctx),
        }
    }
}

impl IntoLaTeX for ResourceDescriptor {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for EmailLink {
    /// `\href{mailto:name@example.org}{name@example.org}`
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for HyperLink {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for ImageLink {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for TagReference {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for TwoWayLink {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}
