//! 共享验收件：实现已上收到 Boost `document-harness`，本模块只保留 `common::` 路径兼容。
#![allow(dead_code, unused_imports)]

pub use document_harness::{fixtures::*, report::*};
pub use document_harness::font_pack::host_font_dir;

/// 加载宿主字体包；缺包直接失败（PDF 写出合同要求，不允许静默跳过）。
pub fn pdf_body_faces() -> std::sync::Arc<document_fonts::BodyFaceSet> {
    document_harness::font_pack::require_pdf_body_faces()
}
