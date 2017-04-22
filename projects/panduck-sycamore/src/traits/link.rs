use super::*;
use notedown_ast::nodes::{EmailLink, HyperLink, ImageLink, ResourceDescriptor, SmartLink, TagReference, TwoWayLink};

impl<G> IntoSycamore<G> for SmartLink
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        match self {
            Self::EMail(v) => v.into_sycamore(ctx),
            Self::Normal(v) => v.into_sycamore(ctx),
            Self::Image(v) => v.into_sycamore(ctx),
            Self::TwoWay(v) => v.into_sycamore(ctx),
            Self::Reference(v) => v.into_sycamore(ctx),
            Self::ExternalResource(v) => v.into_sycamore(ctx),
        }
    }
}

impl<G> IntoSycamore<G> for ResourceDescriptor
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        // <a href="mailto:someone@example.com">Send email</a>
        let a = GenericNode::element("a");
        return a;
    }
}

impl<G> IntoSycamore<G> for EmailLink
where
    G: GenericNode,
{
    /// <a href="mailto:someone@example.com">Send email</a>
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        let a: G = GenericNode::element("a");
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
    fn into_sycamore(self, builder: &SycamoreBuilder) -> G {
        let cfg = &builder.config.image_config;
        let img: G = GenericNode::element("img");
        img.set_attribute("src", &self.src);
        if let Some(s) = self.alt {
            img.set_attribute("alt", &s)
        };
        match cfg.lazy_loading {
            true => img.set_attribute("loading", "lazy"),
            false => {
                // img.set_attribute("loading", "eager") // default, hide
            }
        }
        if false {
            return img;
        }

        let link: G = GenericNode::element("a");
        link.append_child(&img);
        return link;
    }
}

impl<G> IntoSycamore<G> for HyperLink
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        let a = GenericNode::element("a");
        return a;
    }
}

impl<G> IntoSycamore<G> for TwoWayLink
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        todo!()
    }
}
impl<G> IntoSycamore<G> for TagReference
where
    G: GenericNode,
{
    fn into_sycamore(self, _: &SycamoreBuilder) -> G {
        todo!()
    }
}
