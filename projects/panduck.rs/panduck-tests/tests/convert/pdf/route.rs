use document_convert::{convert_bytes, convert_bytes_with_body_faces};
use document_types::AdapterError;

use crate::common;
use common::pdf_body_faces;

#[test]
fn docx_pdf_routes_reopen_text_and_report_reader_losses() {
    let faces = pdf_body_faces();
    let source = "Café € — conversion\n".as_bytes().to_vec();
    let docx = convert_bytes("markdown", "docx", "source.md", source, None).expect("generate DOCX").binary.expect("DOCX bytes");
    let pdf_output = convert_bytes_with_body_faces("docx", "pdf", "source.docx", docx, None, &faces).expect("DOCX to PDF");
    assert!(pdf_output.loss_count > 0);
    let docx_output = convert_bytes("pdf", "docx", "source.pdf", pdf_output.binary.expect("PDF bytes"), None).expect("PDF to DOCX");
    assert!(docx_output.loss_count > 0);
    let reopened = convert_bytes("docx", "markdown", "round.docx", docx_output.binary.expect("DOCX bytes"), None).expect("reopen DOCX");
    assert!(reopened.markdown.contains("Café € — conversion"), "{}", reopened.markdown);
}

#[test]
fn docx_to_pdf_embeds_cjk_body_with_host_faces() {
    let faces = pdf_body_faces();
    let docx = convert_bytes("markdown", "docx", "unicode.md", "中文正文\n".as_bytes().to_vec(), None)
        .expect("generate Unicode DOCX")
        .binary
        .expect("DOCX bytes");
    let output = convert_bytes_with_body_faces("docx", "pdf", "unicode.docx", docx, None, &faces).expect("DOCX to PDF");
    let pdf = output.binary.expect("PDF bytes");
    let reopened = convert_bytes("pdf", "markdown", "unicode-round.pdf", pdf, None).expect("read PDF");
    assert!(reopened.markdown.contains("中文正文"), "{}", reopened.markdown);
}

#[test]
fn pdf_without_host_faces_reports_missing_fonts() {
    let error = convert_bytes("markdown", "pdf", "no-faces.md", b"# Heading\n\nBody text.\n".to_vec(), None)
        .expect_err("PDF writing requires host body faces");
    match error {
        AdapterError::Diagnostic { diagnostic } => {
            assert_eq!(format!("{}", diagnostic.code()), "notedown.convert.pdf-fonts-missing");
        }
        other => panic!("expected a missing-fonts diagnostic, got {other:?}"),
    }
}

#[test]
fn pdf_self_reopen_drops_page_furniture() {
    let faces = pdf_body_faces();
    let mut markdown = String::from("# Document Title\n\n");
    for index in 0..80 {
        markdown.push_str(&format!("Body line {index} of a multi page document.\n\n"));
    }
    let first = convert_bytes_with_body_faces("markdown", "pdf", "self.md", markdown.into_bytes(), None, &faces).expect("write first PDF");
    let first_pdf = first.binary.expect("PDF bytes");
    let second = convert_bytes_with_body_faces("pdf", "pdf", "self.pdf", first_pdf, None, &faces).expect("rewrite PDF");
    let reopened = convert_bytes("pdf", "markdown", "round.pdf", second.binary.expect("PDF bytes"), None).expect("read PDF");
    // The writer paints chrome from page two onward. Each artefact section is
    // dropped on extraction, so the running header and folio never reopen as
    // body text.
    assert_eq!(reopened.markdown.matches("Document Title").count(), 1, "{}", reopened.markdown);
    assert!(reopened.markdown.trim_start().starts_with("# Document Title"), "{}", reopened.markdown);
    assert!(!reopened.markdown.lines().any(|line| line.trim() == "2"), "{}", reopened.markdown);
    assert!(reopened.markdown.contains("Body line 79"), "{}", reopened.markdown);
}

#[test]
fn tex_to_pdf_reopens_three_heading_levels() {
    let faces = pdf_body_faces();
    let tex = "\\section{One}\nBody.\n\n\\subsection{Two}\n\\subsubsection{Three}\n";
    let pdf = convert_bytes_with_body_faces("tex", "pdf", "levels.tex", tex.as_bytes().to_vec(), None, &faces).expect("write PDF");
    let reopened = convert_bytes("pdf", "markdown", "levels.pdf", pdf.binary.expect("PDF bytes"), None).expect("read PDF");
    let markdown = reopened.markdown;
    assert!(markdown.contains("# 1 One"), "{markdown}");
    assert!(markdown.contains("## 1.1 Two"), "{markdown}");
    assert!(markdown.contains("### 1.1.1 Three"), "{markdown}");
}

#[test]
fn markdown_to_pdf_and_back_reopens_text() {
    let faces = pdf_body_faces();
    let output =
        convert_bytes_with_body_faces("markdown", "pdf", "sample.md", b"# Heading\n\nBody text.\n".to_vec(), None, &faces).expect("write PDF");
    let pdf = output.binary.expect("PDF bytes");
    let reopened = convert_bytes("pdf", "markdown", "round.pdf", pdf, None).expect("read PDF");
    assert!(reopened.markdown.contains("Heading"));
    assert!(reopened.markdown.contains("Body text."));
}

#[test]
fn markdown_to_pdf_and_back_preserves_winansi_body_text() {
    let faces = pdf_body_faces();
    let output = convert_bytes_with_body_faces("markdown", "pdf", "latin.md", "Café • € — déjà vu\n".as_bytes().to_vec(), None, &faces)
        .expect("write WinAnsi PDF");
    let pdf = output.binary.expect("PDF bytes");
    let reopened = convert_bytes("pdf", "markdown", "latin-round.pdf", pdf, None).expect("read WinAnsi PDF");
    assert!(reopened.markdown.contains("Café • € — déjà vu"), "{}", reopened.markdown);
}
