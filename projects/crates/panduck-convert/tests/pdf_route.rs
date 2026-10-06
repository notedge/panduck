use panduck_convert::convert_bytes;

#[test]
fn docx_pdf_routes_reopen_text_and_report_reader_losses() {
    let source = "Café € — conversion\n".as_bytes().to_vec();
    let docx = convert_bytes("markdown", "docx", "source.md", source).expect("generate DOCX").binary.expect("DOCX bytes");
    let pdf_output = convert_bytes("docx", "pdf", "source.docx", docx).expect("DOCX to PDF");
    assert!(pdf_output.loss_count > 0);
    let docx_output = convert_bytes("pdf", "docx", "source.pdf", pdf_output.binary.expect("PDF bytes")).expect("PDF to DOCX");
    assert!(docx_output.loss_count > 0);
    let reopened = convert_bytes("docx", "markdown", "round.docx", docx_output.binary.expect("DOCX bytes")).expect("reopen DOCX");
    assert!(reopened.markdown.contains("Café € — conversion"), "{}", reopened.markdown);
}

#[test]
fn docx_to_pdf_rejects_unicode_body_outside_writer_coverage() {
    let docx = convert_bytes("markdown", "docx", "unicode.md", "中文正文\n".as_bytes().to_vec()).expect("generate Unicode DOCX").binary.expect("DOCX bytes");
    assert!(convert_bytes("docx", "pdf", "unicode.docx", docx).is_err());
}

#[test]
fn markdown_to_pdf_and_back_reopens_text() {
    let output = convert_bytes("markdown", "pdf", "sample.md", b"# Heading\n\nBody text.\n".to_vec()).expect("write PDF");
    let pdf = output.binary.expect("PDF bytes");
    let reopened = convert_bytes("pdf", "markdown", "round.pdf", pdf).expect("read PDF");
    assert!(reopened.markdown.contains("Heading"));
    assert!(reopened.markdown.contains("Body text."));
}

#[test]
fn markdown_to_pdf_and_back_preserves_winansi_body_text() {
    let output = convert_bytes("markdown", "pdf", "latin.md", "Café • € — déjà vu\n".as_bytes().to_vec()).expect("write WinAnsi PDF");
    let pdf = output.binary.expect("PDF bytes");
    let reopened = convert_bytes("pdf", "markdown", "latin-round.pdf", pdf).expect("read WinAnsi PDF");
    assert!(reopened.markdown.contains("Café • € — déjà vu"), "{}", reopened.markdown);
}
