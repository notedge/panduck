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
