mod doc;
mod html;
mod markdown;
mod notedown;
mod pdf;

pub use doc::{read_doc, read_doc_bytes};
pub use html::{read_html, read_html_bytes};
pub use markdown::{read_markdown, read_markdown_bytes};
pub use notedown::{read_notedown, read_notedown_bytes};
pub use pdf::{read_pdf, read_pdf_bytes};
