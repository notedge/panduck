//! document.convert 功能验收套件（自 rust-boost 迁入）。

#[path = "convert/route_capabilities/mod.rs"]
mod route_capabilities;

#[path = "convert/common/mod.rs"]
mod common;
#[path = "convert/convert_report/mod.rs"]
mod convert_report;
#[path = "convert/docx/mod.rs"]
mod docx;
#[path = "convert/embed/mod.rs"]
mod embed;
#[path = "convert/epub/mod.rs"]
mod epub;
#[path = "convert/html/mod.rs"]
mod html;
#[cfg(feature = "logger")]
#[path = "convert/logger/mod.rs"]
mod logger;
#[path = "convert/markdown/mod.rs"]
mod markdown;
#[path = "convert/notedown/mod.rs"]
mod notedown;
#[path = "convert/pdf/mod.rs"]
mod pdf;
#[path = "convert/pdf_math_codes.rs"]
mod pdf_math_codes;
#[path = "convert/publish/mod.rs"]
mod publish;
#[path = "convert/rejection_catalog/mod.rs"]
mod rejection_catalog;
#[path = "convert/structure_guard/mod.rs"]
mod structure_guard;
#[path = "convert/transform/mod.rs"]
mod transform;
