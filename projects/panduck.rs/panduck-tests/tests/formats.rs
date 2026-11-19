//! document.formats 功能验收套件（自 rust-boost 迁入）。

#[path = "formats/capability/mod.rs"]
mod capability;
#[cfg(feature = "markdown")]
#[path = "formats/markdown_loss_span/mod.rs"]
mod markdown_loss_span;
#[cfg(feature = "markdown")]
#[path = "formats/markdown_math_ir/mod.rs"]
mod markdown_math_ir;
#[cfg(any(feature = "docx", feature = "html", feature = "epub"))]
#[path = "formats/math_import.rs"]
mod math_import;
#[cfg(feature = "pptx")]
#[path = "formats/pptx.rs"]
mod pptx;
#[cfg(feature = "xlsx")]
#[path = "formats/xlsx.rs"]
mod xlsx;

#[path = "formats/common/mod.rs"]
mod common;
#[cfg(feature = "doc")]
#[path = "formats/doc/mod.rs"]
mod doc;
#[cfg(all(feature = "docx", feature = "markdown"))]
#[path = "formats/docx/mod.rs"]
mod docx;
#[cfg(all(feature = "epub", feature = "markdown"))]
#[path = "formats/epub/mod.rs"]
mod epub;
#[cfg(all(feature = "html", feature = "markdown"))]
#[path = "formats/html/mod.rs"]
mod html;
#[path = "formats/integration/mod.rs"]
mod integration;
#[cfg(feature = "markdown")]
#[path = "formats/markdown/mod.rs"]
mod markdown;
#[cfg(all(feature = "notedown", feature = "markdown", feature = "html"))]
#[path = "formats/notedown/mod.rs"]
mod notedown;
#[cfg(feature = "pandoc-json")]
#[path = "formats/pandoc/mod.rs"]
mod pandoc;
#[cfg(feature = "pdf")]
#[path = "formats/pdf/mod.rs"]
mod pdf;
#[cfg(feature = "markdown")]
#[path = "formats/provenance/mod.rs"]
mod provenance;
#[cfg(feature = "tex")]
#[path = "formats/tex/mod.rs"]
mod tex;
#[cfg(feature = "typst")]
#[path = "formats/typst/mod.rs"]
mod typst;

/// Cross-reference and verbatim constructs that used to fail or drift.
#[cfg(all(feature = "html", feature = "tex"))]
mod tex_export_smoke {
    use document_formats::{export::html::export_html, import::tex::import_tex_bytes};

    #[test]
    fn section_label_ref_resolves_to_the_block_anchor() {
        let graph = import_tex_bytes("label-ref", "\\section{A}\\label{a}\nsee \\ref{a}\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"id="node-1""#), "{html}");
        assert!(html.contains(r##"href="#node-1""##), "{html}");
    }

    #[test]
    fn verbatim_stays_inline_code() {
        let graph = import_tex_bytes("verb", "\\verb|a|\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains("<code>a</code>"), "{html}");
        assert!(!html.contains("<pre>"), "{html}");
    }

    #[test]
    fn sizing_options_are_a_loss_not_a_refusal() {
        let graph = import_tex_bytes("sizing", "\\includegraphics[width=2cm]{a.png}\n").expect("tex import");
        let marker = graph.coverage.loss.iter().find(|marker| marker.code == "import.tex.image_dimensions").expect("sizing loss marker");
        assert_eq!(marker.status, document_ir::SemanticStatus::Partial, "{:?}", graph.coverage.loss);
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"src="a.png""#), "{html}");
    }

    #[test]
    fn environment_wrapper_stays_out_of_the_math_payload() {
        let graph = import_tex_bytes("equation", "\\begin{equation}\nx = 1\n\\end{equation}\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"<div class="math" data-language="latex">x = 1</div>"#), "{html}");
        assert!(!html.contains("\\begin{equation}"), "{html}");
    }

    #[test]
    fn environment_that_carries_the_formula_keeps_its_wrapper() {
        let graph = import_tex_bytes("matrix", "$$\n\\begin{matrix}a&b\\\\c&d\\end{matrix}\n$$\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains("\\begin{matrix}"), "{html}");
    }

    #[test]
    fn footnote_without_a_definition_is_not_a_link() {
        let graph = import_tex_bytes("footnote", "a\\footnote{n}\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"<sup class="footnote-reference" data-footnote-label="1">n</sup>"#), "{html}");
        assert!(!html.contains("href=\"#fn-1\""), "{html}");
    }
}

/// Every local anchor a projection emits must resolve inside the same document.
#[cfg(all(feature = "html", feature = "markdown", feature = "tex", feature = "typst"))]
mod html_anchor_integrity {
    use document_formats::{
        export::html::export_html,
        import::{markdown::import_markdown_bytes, tex::import_tex_bytes, typst::import_typst_bytes},
    };

    fn assert_local_anchors_resolve(label: &str, html: &str) {
        let mut ids: Vec<&str> = Vec::new();
        let mut rest = html;
        while let Some(position) = rest.find("id=\"") {
            let after = &rest[position + 4..];
            let end = after.find('"').expect("unterminated id attribute");
            ids.push(&after[..end]);
            rest = &after[end..];
        }
        for (position, _) in html.match_indices("href=\"#") {
            let after = &html[position + 7..];
            let end = after.find('"').expect("unterminated href attribute");
            let target = &after[..end];
            assert!(ids.contains(&target), "{label}: `href=\"#{target}\"` has no matching `id` in {html}");
        }
    }

    #[test]
    fn projections_link_only_to_existing_anchors() {
        // The HTML importer drops authored `id` attributes because the IR has
        // no field for them, so an `html -> html` round trip can still keep an
        // `href` whose target disappears. That gap is recorded separately and
        // this guard covers the projections that mint their own anchors.
        let cases: Vec<(&str, String)> = vec![
            (
                "markdown",
                export_html(&import_markdown_bytes("anchor-markdown", "see [^n]\n\n[^n]: body\n").expect("markdown import"))
                    .expect("html export"),
            ),
            (
                "tex",
                export_html(&import_tex_bytes("anchor-tex", "\\section{A}\\label{a}\nsee \\ref{a}\n").expect("tex import"))
                    .expect("html export"),
            ),
            ("typst", export_html(&import_typst_bytes("anchor-typst", "= Title\n").expect("typst import")).expect("html export")),
        ];
        for (label, html) in cases {
            assert_local_anchors_resolve(label, &html);
        }
    }
}

/// Constructs the HTML projection degrades or defers instead of failing.
#[cfg(all(feature = "html", feature = "tex"))]
mod tex_local_degradation_smoke {
    use document_formats::{export::html::export_html, import::tex::import_tex_bytes};
    use document_ir::DocumentGraph;

    fn has_loss(graph: &DocumentGraph, code: &str) -> bool {
        graph.coverage.loss.iter().any(|marker| marker.code == code)
    }

    #[test]
    fn citation_degrades_to_its_key() {
        let graph = import_tex_bytes("cite", "\\cite{a}\n").expect("tex import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"data-citation-key="a""#), "{html}");
        assert!(html.contains("[a]"), "{html}");
    }

    #[test]
    fn macro_definition_stays_visible_and_recorded() {
        let graph = import_tex_bytes("newcommand", "\\newcommand{\\x}{y}\nHello\n").expect("tex import");
        assert!(has_loss(&graph, "import.tex.macro_definition"), "{:?}", graph.coverage.loss);
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"data-opaque-kind="tex.newcommand""#), "{html}");
        assert!(html.contains("Hello"), "{html}");
    }
}

/// Typst dynamic constructs are skipped locally, so the rest of the page still
/// exports and the deferral stays in the loss markers.
#[cfg(all(feature = "html", feature = "typst"))]
mod typst_local_degradation_smoke {
    use document_formats::{export::html::export_html, import::typst::import_typst_bytes};

    #[test]
    fn dynamic_expression_does_not_fail_the_document() {
        let graph = import_typst_bytes("dynamic", "#let x = 1\nHello\n").expect("typst import");
        assert!(graph.coverage.loss.iter().any(|marker| marker.code == "import.typst.dynamic_content"));
        let html = export_html(&graph).expect("html export");
        assert!(html.contains("Hello"), "{html}");
        assert!(!html.contains("#let"), "{html}");
    }
}

/// HTML structures the importer used to lose or merge.
#[cfg(feature = "html")]
mod html_structure_smoke {
    use document_formats::{export::html::export_html, import::html::import_html_text};
    use document_ir::Block;

    fn html(source: &str) -> String {
        let graph = import_html_text("html-structure", source).expect("html import");
        export_html(&graph).expect("html export")
    }

    fn table(
        graph: &document_ir::DocumentGraph,
    ) -> (&Option<String>, &[document_ir::TableRow], &[document_ir::TableRow], &[document_ir::TableRow]) {
        graph
            .blocks
            .iter()
            .find_map(|node| match &node.block {
                Block::Table { caption, head, body, foot } => Some((caption, head.as_slice(), body.as_slice(), foot.as_slice())),
                _ => None,
            })
            .unwrap_or_else(|| panic!("table expected: {:#?}", graph.blocks))
    }

    fn loss_codes(source: &str) -> Vec<String> {
        let graph = import_html_text("html-structure", source).expect("html import");
        graph.coverage.loss.iter().map(|marker| marker.code.clone()).collect()
    }

    #[test]
    fn unquoted_attribute_value_stays_in_the_tag() {
        let output = html("<a href=/x>x</a>");
        assert!(output.contains(r#"href="/x""#), "{output}");
    }

    #[test]
    fn boolean_attribute_keeps_its_spelling() {
        let output = html(r#"<a hidden href="/x">x</a>"#);
        assert!(output.contains("hidden"), "{output}");
    }

    #[test]
    fn table_sections_keep_their_cells() {
        let output =
            html("<table><colgroup><col span=2></colgroup><thead><tr><th>h</th></tr></thead><tbody><tr><td>b</td></tr></tbody></table>");
        assert!(output.contains("<thead><tr><th><p>h</p></th></tr></thead>"), "{output}");
        assert!(output.contains("<tbody><tr><td><p>b</p></td></tr></tbody>"), "{output}");
        assert!(
            !loss_codes("<table><thead><tr><th>h</th></tr></thead><tbody><tr><td>b</td></tr></tbody></table>")
                .contains(&"import.html.table_sections".to_string())
        );
    }

    #[test]
    fn header_rows_after_body_rows_are_a_recorded_loss() {
        let output = html("<table><tbody><tr><td>b</td></tr><tr><th>h</th></tr></tbody></table>");
        // A loose `<th>` row inside a body section after a body row cannot move
        // into the head band, so it is written as body content and the regroup
        // stays a recorded loss.
        assert!(output.contains("<td><p>h</p></td>"), "{output}");
        assert!(
            loss_codes("<table><tbody><tr><td>b</td></tr><tr><th>h</th></tr></tbody></table>")
                .contains(&"import.html.table_sections".to_string())
        );
    }

    #[test]
    fn late_head_sections_are_recorded_and_leading_heads_are_not() {
        let late = "<table><tbody><tr><td>Body</td></tr></tbody><thead><tr><th>Late</th></tr></thead></table>";
        let codes = loss_codes(late);
        assert!(codes.contains(&"import.html.head_after_body".to_string()), "{codes:?}");
        assert!(!codes.contains(&"import.html.table_sections".to_string()), "a late head section is not a scattered `th` row: {codes:?}");
        let output = html(late);
        assert!(output.contains("<thead><tr><th><p>Late</p></th></tr></thead>"), "{output}");
        assert!(output.contains("<tbody><tr><td><p>Body</p></td></tr></tbody>"), "{output}");

        let leading = "<table><thead><tr><th>Head</th></tr></thead><tbody><tr><td>Body</td></tr></tbody></table>";
        let codes = loss_codes(leading);
        assert!(!codes.contains(&"import.html.head_after_body".to_string()), "{codes:?}");
    }

    #[test]
    fn table_bands_and_cell_children_survive_export_and_reimport() {
        let source = concat!(
            "<table><caption>Cap</caption>",
            "<thead><tr><th>h1</th><th>h2</th></tr></thead>",
            "<tbody><tr><td><p>a</p><p>b</p></td><td>c</td></tr></tbody>",
            "<tfoot><tr><td>f</td><td>g</td></tr></tfoot></table>"
        );
        let graph = import_html_text("html-table-bands", source).expect("html import");
        let (caption, head, body, foot) = table(&graph);
        assert_eq!(caption.as_deref(), Some("Cap"));
        assert_eq!((head.len(), body.len(), foot.len()), (1, 1, 1));
        assert!(!loss_codes(source).contains(&"import.html.table_cell_blocks".to_string()), "cell block layout is preserved, not flattened");
        let output = export_html(&graph).expect("html export");
        assert!(output.contains("<thead><tr><th><p>h1</p></th><th><p>h2</p></th></tr></thead>"), "{output}");
        assert!(output.contains("<tbody><tr><td><p>a</p><p>b</p></td><td><p>c</p></td></tr></tbody>"), "{output}");
        assert!(output.contains("<tfoot><tr><td><p>f</p></td><td><p>g</p></td></tr></tfoot>"), "{output}");

        let reopened = import_html_text("html-table-bands-roundtrip", &output).expect("html reimport");
        let (_, head, body, foot) = table(&reopened);
        assert_eq!((head.len(), body.len(), foot.len()), (1, 1, 1));
    }

    #[test]
    fn table_export_keeps_colspan_and_rowspan_in_the_band_that_owns_them() {
        use document_ir::{DocumentGraph, DocumentId, Inline, TableCell, TableRow};

        let mut graph = DocumentGraph::new(DocumentId(9));
        let heading = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "h".into() }] });
        let tall = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "a".into() }] });
        let b = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "b".into() }] });
        let c = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "c".into() }] });
        graph.push_block(Block::Table {
            caption: None,
            head: vec![TableRow { cells: vec![TableCell { children: vec![heading], colspan: 2, rowspan: 1 }] }],
            body: vec![
                TableRow {
                    cells: vec![
                        TableCell { children: vec![tall], colspan: 1, rowspan: 2 },
                        TableCell { children: vec![b], colspan: 1, rowspan: 1 },
                    ],
                },
                TableRow { cells: vec![TableCell { children: vec![c], colspan: 1, rowspan: 1 }] },
            ],
            foot: Vec::new(),
        });
        let html = export_html(&graph).expect("html export with spans");
        assert!(html.contains(r#"<th colspan="2">"#), "{html}");
        assert!(html.contains(r#"<td rowspan="2">"#), "{html}");
    }

    #[test]
    fn foster_parented_text_precedes_the_table() {
        let output = html("<table>stray<tr><td>x</td></tr></table>");
        assert!(output.contains("<p>stray</p>"), "{output}");
    }

    #[test]
    fn implied_paragraph_close_splits_paragraphs() {
        let output = html("<p>a<p>b");
        assert_eq!(output.matches("<p>").count(), 2, "{output}");
    }

    #[test]
    fn implied_list_item_close_splits_items() {
        let output = html("<ul><li>a<li>b</ul>");
        assert!(output.contains("<li>a</li>"), "{output}");
        assert!(output.contains("<li>b</li>"), "{output}");
    }

    #[test]
    fn template_content_stays_inert() {
        let output = html("<template><p>x</p></template>");
        assert!(output.contains("<template></template>"), "{output}");
        assert!(!output.contains("<p>x</p>"), "{output}");
    }

    #[test]
    fn trailing_text_after_self_closed_non_void_element_survives() {
        let output = html("<div></div>after");
        assert!(output.contains("after"), "{output}");
    }

    #[test]
    fn script_content_does_not_swallow_following_siblings() {
        let source = "<script>if (a < b) { x() }</script><p>after</p>";
        let output = html(source);
        assert!(output.contains("<p>after</p>"), "{output}");
        assert!(output.contains("<script>if (a < b) { x() }</script>"), "{output}");
        assert!(loss_codes(source).contains(&"import.html.raw_script".to_string()));
    }
}

/// A link whose text is an image keeps both the outer link and the image.
#[cfg(all(feature = "html", feature = "markdown"))]
mod markdown_export_smoke {
    use document_formats::{export::html::export_html, import::markdown::import_markdown_bytes};

    #[test]
    fn link_text_image_is_not_truncated() {
        let graph = import_markdown_bytes("nested-image", "[![a](i.png)](/u)\n").expect("markdown import");
        let html = export_html(&graph).expect("html export");
        assert!(html.contains(r#"href="/u""#), "{html}");
        assert!(html.contains(r#"src="i.png""#), "{html}");
        assert!(html.contains(r#"alt="a""#), "{html}");
    }
}

/// Loss status classification: kept content is a loss, dropped content is a
/// whole-construct refusal.
#[cfg(all(feature = "html", feature = "tex", feature = "typst"))]
mod loss_status_smoke {
    use document_formats::{
        export::html::export_html,
        import::{html::import_html_bytes, tex::import_tex_bytes, typst::import_typst_bytes},
    };
    use document_ir::{Block, DocumentGraph, SemanticStatus};

    fn status(graph: &DocumentGraph, code: &str) -> SemanticStatus {
        graph.coverage.loss.iter().find(|loss| loss.code == code).unwrap_or_else(|| panic!("missing loss `{code}`")).status
    }

    /// The opaque block a marker describes carries the same status as the marker,
    /// so the node never claims a mapping the projection skips.
    fn opaque_status(graph: &DocumentGraph, prefix: &str) -> SemanticStatus {
        graph
            .blocks
            .iter()
            .find_map(|node| match &node.block {
                Block::Opaque { kind, status, .. } if kind.starts_with(prefix) => Some(*status),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing `{prefix}` opaque block"))
    }

    #[test]
    fn kept_tex_content_is_a_loss_not_a_refusal() {
        let pair = import_tex_bytes("left-pair", "\\left[ a\n").expect("tex import");
        assert_eq!(status(&pair, "import.tex.unclosed_delimiters"), SemanticStatus::Partial);
        assert!(export_html(&pair).expect("html export").contains("tex-delimiter-pair"));

        let math = import_tex_bytes("unclosed-math", "$a\n").expect("tex import");
        assert_eq!(status(&math, "import.tex.unclosed_math"), SemanticStatus::Partial);
        assert!(export_html(&math).expect("html export").contains(r#"class="math""#));

        let macro_graph = import_tex_bytes("macro", "\\newcommand{\\foo}{bar}\ntext\n").expect("tex import");
        assert_eq!(status(&macro_graph, "import.tex.macro_definition"), SemanticStatus::Partial);
        assert_eq!(opaque_status(&macro_graph, "tex."), SemanticStatus::Partial);
        assert!(export_html(&macro_graph).expect("html export").contains(r#"data-opaque-kind="tex.newcommand""#));
    }

    #[test]
    fn dropped_content_stays_a_refusal() {
        let typst = import_typst_bytes("let", "#let x = 1\nhello\n").expect("typst import");
        assert_eq!(status(&typst, "import.typst.dynamic_content"), SemanticStatus::Unsupported);
        assert_eq!(opaque_status(&typst, "typst."), SemanticStatus::Unsupported);
        let typst_html = export_html(&typst).expect("html export");
        assert!(!typst_html.contains("#let"), "{typst_html}");

        let template = import_html_bytes("template", b"<template><p>x</p></template>").expect("html import");
        assert_eq!(status(&template, "import.html.template_contents"), SemanticStatus::Unsupported);
        let template_html = export_html(&template).expect("html export");
        assert!(template_html.contains("<template></template>"), "{template_html}");
        assert!(!template_html.contains("<p>x</p>"), "{template_html}");
    }
}
