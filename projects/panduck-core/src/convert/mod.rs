#[cfg(feature = "docx-rs")]
mod docx;

#[cfg(feature = "html")]
mod html;
#[cfg(feature = "html")]
pub use self::html::{parse_html, register_html};

#[cfg(feature = "jupyter")]
mod jupyter;
#[cfg(feature = "jupyter")]
pub use self::jupyter::{parse_jupyter, register_jupyter};

// #[cfg(feature = "markdown")]
// mod markdown;
#[cfg(feature = "markdown")]
mod common_markdown;
#[cfg(feature = "markdown")]
mod pandoc_markdown;
#[cfg(feature = "markdown")]
pub use self::common_markdown::{parse_common_markdown, register_common_markdown};

#[cfg(feature = "notedown")]
mod notedown;
#[cfg(feature = "rtf")]
mod rich_text;
#[cfg(feature = "wiki")]
mod wiki;

#[cfg(feature = "notedown")]
pub use notedown::register_notedown;

#[cfg(feature = "rst")]
mod rst;
#[cfg(feature = "rst")]
pub use self::rst::{parse_rst, register_rst};

use notedown_ast::{ASTNode, ASTNodes};

pub trait ToNotedown
where
    Self: Sized,
{
    fn into_notedown(self) -> ASTNode;
    fn into_notedown_list(self) -> ASTNodes {
        vec![self.into_notedown()]
    }
}
