use document_formats::import::doc::import_doc_bytes;

#[test]
fn rejects_non_ole_legacy_document() {
    let error = import_doc_bytes("not-a-doc", b"plain text").expect_err("invalid OLE should fail");
    assert!(error.to_string().contains("OLE"));
}
mod import;
