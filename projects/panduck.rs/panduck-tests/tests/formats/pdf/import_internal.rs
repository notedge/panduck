use std::collections::HashMap;

use document_formats::import::pdf::test_support::{
    decode_pdf_string, decode_text_bytes, decode_with_font, increment_unicode, parse_inline_jpeg, parse_to_unicode_cmap, split_pdf_paragraphs,
};

#[test]
fn preserves_leading_and_trailing_spaces_in_pdf_text_paragraphs() {
    let paragraphs = split_pdf_paragraphs("  leading\ttext  \nnext  ");
    assert_eq!(paragraphs, ["  leading\ttext  ", "next  "]);
}

#[test]
fn literal_escapes_are_decoded_before_tounicode_mapping() {
    let map = parse_to_unicode_cmap(b"2 beginbfchar\n<01> <4F60>\n<02> <597D>\nendbfchar").expect("CMap");
    let maps = HashMap::from([("F1".into(), map)]);
    assert_eq!(decode_with_font(br"\001\002", Some("F1"), &maps, true), "你好");
    assert_eq!(decode_with_font(&[1, 2], Some("F1"), &maps, false), "你好");
}

#[test]
fn decodes_pdf_doc_encoding_and_line_continuations() {
    assert_eq!(decode_text_bytes(&[b'A', 0x80, 0x9f]), "A•€");
    assert_eq!(decode_pdf_string(b"line\\\ncontinued"), "linecontinued");
    assert_eq!(decode_pdf_string(b"line\\\r\ncontinued"), "linecontinued");
}

#[test]
fn inline_jpeg_ignores_operator_like_bytes_inside_image() {
    let stream = b"BI /F /DCTDecode ID \xFF\xD8 payload EI (fake) Tj \xFF\xD9\nEI\nBT (real) Tj ET";
    let (image, end) = parse_inline_jpeg(stream, 0).expect("inline JPEG");
    assert_eq!(image.bytes, b"\xFF\xD8 payload EI (fake) Tj \xFF\xD9");
    assert_eq!(&stream[end..], b"\nBT (real) Tj ET");
}

#[test]
fn inline_jpeg_requires_eoi_and_complete_operator_boundaries() {
    for stream in [
        &b"BI /F /DCTDecode ID \xFF\xD8 data EI "[..],
        &b"BI /F /DCTDecode ID \xFF\xD8\xFF\xD9 EIx "[..],
        &b"BI /F /DCTDecode IDx \xFF\xD8\xFF\xD9 EI "[..],
    ] {
        assert!(parse_inline_jpeg(stream, 0).is_none());
    }
}

#[test]
fn decodes_surrogate_pair_bfrange_targets() {
    let map = parse_to_unicode_cmap(b"1 beginbfrange\n<01> <03> <D83DDE00>\nendbfrange").expect("CMap");
    assert_eq!(map.decode(&[1, 2, 3]), "😀😁😂");
}

#[test]
fn decodes_multiple_code_units_in_range_targets() {
    let map = parse_to_unicode_cmap(b"1 beginbfrange\n<01> <02> <00660069>\nendbfrange").expect("CMap");
    assert_eq!(map.decode(&[1, 2]), "fifj");
}

#[test]
fn codespace_ranges_control_multibyte_grouping() {
    let map = parse_to_unicode_cmap(b"1 begincodespacerange\n<0000> <00FF>\nendcodespacerange\n1 beginbfchar\n<0102> <4F60>\nendbfchar")
        .expect("CMap");
    assert_eq!(map.decode(&[1, 2]), "你");
    assert_eq!(map.decode(&[1, 2, 3]), "你\u{FFFD}");
}

#[test]
fn rejects_reversed_and_unbounded_ranges() {
    assert!(parse_to_unicode_cmap(b"1 beginbfrange\n<02> <01> <0041>\nendbfrange").is_none());
    assert!(parse_to_unicode_cmap(b"1 beginbfrange\n<00000000> <FFFFFFFF> <0041>\nendbfrange").is_none());
}

#[test]
fn range_increment_carries_and_reports_overflow() {
    let mut value = vec![0x00, 0x01, 0xFF, 0xFF];
    assert!(increment_unicode(&mut value));
    assert_eq!(value, [0x00, 0x02, 0x00, 0x00]);
    assert!(!increment_unicode(&mut vec![0xFF, 0xFF]));
}
