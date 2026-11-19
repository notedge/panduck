//! 跨格式集成（按 feature 门控子模块）。

#[cfg(all(feature = "markdown", feature = "html"))]
mod chart_block;
#[cfg(all(feature = "markdown", feature = "html", feature = "docx", feature = "pdf"))]
mod diagram_fidelity;
#[cfg(all(feature = "markdown", feature = "html"))]
mod diagram_scene;
#[cfg(all(feature = "markdown", feature = "html"))]
mod engraving_depiction_block;
#[cfg(all(feature = "epub", feature = "html", feature = "markdown"))]
mod epub_html_roundtrip;
#[cfg(all(feature = "markdown", feature = "docx"))]
mod markdown_to_docx;
#[cfg(all(feature = "markdown", feature = "epub"))]
mod markdown_to_epub;
#[cfg(all(feature = "markdown", feature = "epub", feature = "notedown"))]
mod markdown_to_epub_export;
