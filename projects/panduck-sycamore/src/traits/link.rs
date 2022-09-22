use notedown_ast::nodes::{EmailLink, HyperLink, ImageLink, ResourceDescriptor, SmartLink, TagReference, TwoWayLink};

use super::*;

impl<G> IntoSycamore<G> for SmartLink
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        match self {
            Self::EMail(v) => v.into_sycamore(cfg, ctx),
            Self::Normal(v) => v.into_sycamore(cfg, ctx),
            Self::Image(v) => v.into_sycamore(cfg, ctx),
            Self::TwoWay(v) => v.into_sycamore(cfg, ctx),
            Self::Reference(v) => v.into_sycamore(cfg, ctx),
            Self::ExternalResource(v) => v.into_sycamore(cfg, ctx),
        }
    }
}

impl<G> IntoSycamore<G> for ResourceDescriptor
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        // <a href="mailto:someone@example.com">Send email</a>
        let a = GenericNode::element_from_tag("a");
        return a;
    }
}

impl<G> IntoSycamore<G> for EmailLink
where
    G: GenericNode,
{
    /// <a href="mailto:someone@example.com">Send email</a>
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let a: G = GenericNode::element_from_tag("a");
        a.set_attribute("href", "mailto:someone@example.com");
        a.update_inner_text("mailto:someone@example.com");
        return a;
    }
}

impl<G> IntoSycamore<G> for ImageLink
where
    G: GenericNode,
{
    ///
    /// <a href="www.baidu.com">
    //     <img border="0" src="/i/eg_buttonnext.gif" />
    // </a>

    ///
    /// <img class="fit-picture"
    //      src="/media/cc0-images/grapefruit-slice-332-332.jpg"
    //      alt="Grapefruit slice atop a pile of other slices">

    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let cfg = &cfg.image_config;
        let img: G = GenericNode::element_from_tag("img");
        img.set_attribute("src", &self.source);
        if let Some(s) = self.description {
            img.set_attribute("alt", &s)
        };
        if let true = cfg.lazy_loading {
            // img.set_attribute("loading", "eager") // default, hide
            img.set_attribute("loading", "lazy")
        }
        let link = match self.link {
            None => return img,
            Some(s) => {
                let link: G = GenericNode::element_from_tag("a");
                link.append_child(&img);
                link.set_attribute("src", &s);
                link
            }
        };

        return link;
    }
}

impl<G> IntoSycamore<G> for HyperLink
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        let a: G = GenericNode::element_from_tag("a");
        a.set_attribute("href", &self.src);
        self.text.map(|f| a.update_inner_text(&f));
        return a;
    }
}

impl<G> IntoSycamore<G> for TwoWayLink
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        todo!()
    }
}
impl<G> IntoSycamore<G> for TagReference
where
    G: GenericNode,
{
    fn into_sycamore(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> G {
        todo!()
    }
}
