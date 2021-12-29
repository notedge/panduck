use notedown_ast::{nodes::StyleKind, ASTKind, ASTNode};

use panduck_pp::*;

use crate::{HTMLConfig, HTMLContext};

mod nodes;
mod text;

pub trait IntoHTML {
    //noinspection RsSelfConvention
    fn into_html<'a>(&'a self, cfg: &HTMLConfig, ctx: &mut HTMLContext) -> PrettyPrint<'a>;
}
