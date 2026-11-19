use document_formats::{
    export::html::export_html,
    import::html::{HtmlLinkRole, import_html_text, import_html_with_links},
};
use document_ir::{Block, BreakKind, DocumentGraph, DocumentId, Inline, RelationKind, SemanticStatus, SourceKind};

fn table(graph: &DocumentGraph) -> (&Option<String>, &[document_ir::TableRow], &[document_ir::TableRow], &[document_ir::TableRow]) {
    graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { caption, head, body, foot } => Some((caption, head.as_slice(), body.as_slice(), foot.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("table expected, got {:?}", graph.blocks))
}

fn cell_text(graph: &DocumentGraph, cell: &document_ir::TableCell) -> String {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

fn quote_paragraph(graph: &DocumentGraph) -> &[Inline] {
    let children = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Quote { children } => Some(children.as_slice()),
            _ => None,
        })
        .expect("quote expected");
    children
        .iter()
        .find_map(|child_id| match graph.block(*child_id).map(|node| &node.block) {
            Some(Block::Paragraph { content }) => Some(content.as_slice()),
            _ => None,
        })
        .expect("quote body must be a paragraph child")
}

#[test]
fn html_export_writes_an_unknown_inline_style_as_a_class() {
    let mut graph = DocumentGraph::new(DocumentId(78));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "zzstyle".into(), children: vec![Inline::Text { text: "ZZR53ZZ".into() }] }],
    });

    let html = export_html(&graph).expect("an unknown style must not reject the HTML export");
    assert!(html.contains(r#"<span class="zzstyle">ZZR53ZZ</span>"#), "{html}");
}

#[test]
fn html_export_of_a_known_style_writes_no_class() {
    let mut graph = DocumentGraph::new(DocumentId(79));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "bold".into(), children: vec![Inline::Text { text: "ZZ_BOLD_ZZ".into() }] }],
    });

    let html = export_html(&graph).expect("export bold run");
    assert!(html.contains("<strong>ZZ_BOLD_ZZ</strong>"), "{html}");
    assert!(!html.contains(r#"class="bold""#), "a known style needs no class: {html}");
}

#[test]
fn html_blockquote_round_trips_quoted_text() {
    let source = "<body><blockquote><p>Quoted line</p></blockquote></body>";
    let graph = import_html_text("quote.html", source).expect("import html quote");
    let content = quote_paragraph(&graph);
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "Quoted line");

    let html = export_html(&graph).expect("export html quote");
    assert!(html.contains("<blockquote>") && html.contains("<p>Quoted line</p>"), "{html}");

    let reopened = import_html_text("quote-roundtrip.html", &html).expect("reopen html quote");
    let content = quote_paragraph(&reopened);
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "Quoted line");
}

#[test]
fn html_diagram_round_trips_through_the_diagram_attribute() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block(Block::Diagram { language: "mermaid".into(), source: "flowchart TD\n  A --> B".into() });

    let html = export_html(&graph).expect("export html");
    assert!(html.contains(r#"data-diagram-language="mermaid""#), "{html}");
    let reopened = import_html_text("diagram.html", &html).expect("import html");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
}

#[test]
fn html_round_trips_style_script_and_form_markup() {
    let source = concat!(
        "<html><head><title>Styled</title>",
        "<style>.lead { color: rebeccapurple }</style>",
        "<script>window.flag = true</script>",
        "</head><body>",
        "<p class=\"lead\">Intro</p>",
        "<form action=\"/search\"><input name=\"q\"/><button type=\"submit\">Go</button></form>",
        "<script>if (a < b) { console.log(\"lt\") }</script>",
        "<p>after</p></body></html>"
    );
    let imported = import_html_text("styled.html", source).expect("import html");
    assert_eq!(imported.metadata.title.as_deref(), Some("Styled"));
    let html = export_html(&imported).expect("export html");
    assert!(html.contains("<style>.lead { color: rebeccapurple }</style>"), "{html}");
    assert!(html.contains("<script>window.flag = true</script>"), "{html}");
    assert!(html.contains("<form action=\"/search\"><input name=\"q\"/><button type=\"submit\">Go</button></form>"), "{html}");
    assert!(html.contains("<script>if (a < b) { console.log(\"lt\") }</script>"), "{html}");
    assert!(html.contains("<p>after</p>"), "{html}");
    let codes: Vec<&str> = imported.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    for code in ["import.html.raw_style", "import.html.raw_script", "import.html.raw_form", "import.html.raw_form_control"] {
        assert!(codes.contains(&code), "missing {code}: {codes:?}");
    }
    assert!(!codes.contains(&"import.html.unsupported_style"), "{codes:?}");
    for (index, loss) in imported.coverage.loss.iter().enumerate() {
        if loss.code.starts_with("import.html.raw_") {
            let span = imported.loss_span(index).unwrap_or_else(|| panic!("{} must carry an authored span", loss.code));
            assert!(span.end > span.start, "{} spans an empty range: {span:?}", loss.code);
            assert!(source.get(span.start as usize..span.end as usize).is_some(), "{} span points outside the source: {span:?}", loss.code);
        }
    }
}

#[test]
fn html_import_reads_rendered_diagram_figures_without_leaking_svg_text() {
    let source = "flowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]";
    let hex = source.bytes().map(|byte| format!("{byte:02x}")).collect::<String>();
    let html = format!(
        "<body><article><figure class=\"diagram\" data-diagram-language=\"mermaid\" data-diagram-source-hex=\"{hex}\"><svg viewBox=\"0 0 10 10\"><text>rendered label</text></svg></figure></article></body>"
    );

    let reopened = import_html_text("rendered-diagram.html", &html).expect("import rendered figure");
    assert_eq!(single_diagram(&reopened), ("mermaid".to_string(), source.to_string()));
    assert!(
        !reopened.blocks.iter().any(|node| match &node.block {
            Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<String>().contains("rendered label"),
            _ => false,
        }),
        "svg payload text must not leak into paragraphs: {:?}",
        reopened.blocks
    );
}

fn single_diagram(graph: &DocumentGraph) -> (String, String) {
    let diagrams = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Diagram { language, source } => Some((language.clone(), source.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(diagrams.len(), 1, "exactly one diagram expected: {:?}", graph.blocks);
    diagrams.into_iter().next().unwrap()
}

#[test]
fn html_import_promotes_mermaid_container_classes_to_diagram_blocks() {
    let flowchart = "flowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]";

    let div = import_html_text("mermaid-div.html", &format!("<body><article><div class=\"mermaid\">\n{flowchart}\n</div></article></body>"))
        .expect("import div container");
    assert_eq!(single_diagram(&div), ("mermaid".to_string(), flowchart.to_string()));

    let pre = import_html_text("mermaid-pre.html", &format!("<body><pre class=\"mermaid\">\n{flowchart}\n</pre></body>")).expect("import pre");
    assert_eq!(single_diagram(&pre), ("mermaid".to_string(), flowchart.to_string()));

    let fenced =
        import_html_text("mermaid-fenced.html", &format!("<body><pre><code class=\"language-mermaid\">{flowchart}</code></pre></body>"))
            .expect("import fenced code");
    assert_eq!(single_diagram(&fenced), ("mermaid".to_string(), flowchart.to_string()));

    let shouted =
        import_html_text("mermaid-shouted.html", &format!("<body><pre><code class=\"LANGUAGE-MERMAID\">{flowchart}</code></pre></body>"))
            .expect("import upper-case marker");
    assert_eq!(single_diagram(&shouted), ("mermaid".to_string(), flowchart.to_string()));
}

#[test]
fn html_import_keeps_image_only_mermaid_container_as_bitmap_with_loss() {
    let graph = import_html_text(
        "mermaid-bitmap.html",
        r#"<body><div class="mermaid"><img src="assets/flow.png" alt="flowchart" width="640" height="480"></div></body>"#,
    )
    .expect("import image-only mermaid container");

    assert!(
        !graph.blocks.iter().any(|node| matches!(node.block, Block::Diagram { .. })),
        "an image-only container must not be promoted to an empty diagram: {:?}",
        graph.blocks
    );

    let image = graph
        .blocks
        .iter()
        .flat_map(|node| block_images(&node.block))
        .next()
        .unwrap_or_else(|| panic!("the bitmap must survive as an image: {:?}", graph.blocks));
    assert_eq!(image.0, "assets/flow.png");
    // The authored display size stays on the image wrapper as attribute pairs,
    // so it reaches exporters as a real `<img width height>` instead of a loss.
    assert_eq!(
        image.1,
        vec![("width".to_string(), "640".to_string()), ("height".to_string(), "480".to_string())],
        "the bitmap keeps its display size: {:?}",
        image.1
    );
    assert!(
        !graph.coverage.loss.iter().any(|loss| loss.code == "import.html.image_dimensions"),
        "a display size no longer needs a loss: {:?}",
        graph.coverage.loss
    );

    assert_eq!(graph.assets.len(), 1, "the bitmap must be registered as an asset: {:?}", graph.assets);
    assert_eq!(graph.assets[0].source.as_deref(), Some("assets/flow.png"));
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.html.empty_diagram_container" && loss.status == SemanticStatus::Partial),
        "the empty container must surface a structured loss: {:?}",
        graph.coverage.loss
    );
}

#[test]
fn html_import_keeps_image_only_mermaid_pre_on_the_default_path() {
    let graph = import_html_text("mermaid-pre-bitmap.html", r#"<body><pre class="mermaid"><img src="chart.png" alt="chart"></pre></body>"#)
        .expect("import image-only mermaid pre");

    assert!(
        !graph.blocks.iter().any(|node| matches!(node.block, Block::Diagram { .. } | Block::Code { .. })),
        "an image-only pre must be neither an empty diagram nor an empty code block: {:?}",
        graph.blocks
    );
    assert_eq!(graph.assets.len(), 1, "the bitmap must be registered as an asset: {:?}", graph.assets);
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.html.empty_diagram_container"));
}

#[test]
fn html_import_keeps_plain_images_and_textual_mermaid_containers_unchanged() {
    // Plain images keep their exact behavior: no diagram promotion, no empty
    // container loss, and no attribute wrapper when no size was authored.
    let plain = import_html_text("plain-image.html", r#"<body><p><img src="a.png" alt="A"></p></body>"#).expect("import plain image");
    let Block::Paragraph { content } = &plain.blocks[0].block
    else {
        panic!("expected a paragraph: {:?}", plain.blocks)
    };
    assert!(matches!(&content[0], Inline::Image { source, .. } if source == "a.png"), "a size-less image stays unwrapped: {:?}", content);
    assert!(!plain.coverage.loss.iter().any(|loss| loss.code == "import.html.empty_diagram_container"));

    // Textual mermaid containers still promote with their authored source.
    let flowchart = "flowchart TD\n  A --> B";
    let textual = import_html_text("mermaid-textual.html", &format!("<body><div class=\"mermaid\">\n{flowchart}\n</div></body>"))
        .expect("import textual");
    assert_eq!(single_diagram(&textual), ("mermaid".to_string(), flowchart.to_string()));
    assert!(!textual.coverage.loss.iter().any(|loss| loss.code == "import.html.empty_diagram_container"));
}

/// Collect every image in a block together with its attribute pairs,
/// descending through `Attributed` wrappers.
fn block_images(block: &Block) -> Vec<(String, Vec<(String, String)>)> {
    let mut images = Vec::new();
    let content: &[Inline] = match block {
        Block::Paragraph { content } => content,
        _ => &[],
    };
    collect_inline_images(content, &[], &mut images);
    images
}

fn collect_inline_images(inlines: &[Inline], inherited: &[(String, String)], images: &mut Vec<(String, Vec<(String, String)>)>) {
    for inline in inlines {
        match inline {
            Inline::Image { source, .. } => images.push((source.clone(), inherited.to_vec())),
            Inline::Attributed { attributes, children } => {
                let pairs = attributes.pairs.iter().map(|pair| (pair.key.clone(), pair.value.clone())).collect::<Vec<_>>();
                collect_inline_images(children, &pairs, images);
            }
            Inline::Styled { children, .. } => collect_inline_images(children, inherited, images),
            _ => {}
        }
    }
}

#[test]
fn html_import_keeps_inline_svg_as_raw_markup_without_leaking_label_text() {
    let graph = import_html_text(
        "inline-svg.html",
        r#"<body><p>Status <svg viewBox="0 0 10 10" width="10" height="10"><circle cx="5" cy="5" r="4"/><text>icon</text></svg> online</p></body>"#,
    )
    .expect("import inline svg");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected, got {:?}", graph.blocks)
    };
    let svg = content.iter().find_map(|inline| match inline {
        Inline::RawHtml { html } => Some(html.clone()),
        _ => None,
    });
    let Some(svg) = svg
    else {
        panic!("inline svg must stay raw markup, got {content:?}")
    };
    assert_eq!(svg, r#"<svg viewBox="0 0 10 10" width="10" height="10"><circle cx="5" cy="5" r="4"/><text>icon</text></svg>"#);
    let prose = content.iter().map(Inline::plain_text).collect::<String>();
    assert!(!prose.contains("icon"), "svg label text must not leak into prose: {prose:?}");
    assert!(prose.contains("Status") && prose.contains("online"), "{prose:?}");
}

#[test]
fn html_import_keeps_block_svg_verbatim_and_reopens_it() {
    let markup = r#"<svg viewBox="0 0 100 40"><rect width="100" height="40"/><text x="4" y="20">gauge</text></svg>"#;
    let graph = import_html_text("block-svg.html", &format!("<body><p>before</p><figure>{markup}</figure><p>after</p></body>"))
        .expect("import block svg");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("leading paragraph expected, got {:?}", graph.blocks)
    };
    assert!(content.iter().map(Inline::plain_text).collect::<String>().contains("before"));
    let Block::RawHtml { html } = &graph.blocks[1].block
    else {
        panic!("raw html block expected, got {:?}", graph.blocks)
    };
    assert_eq!(html, markup);
    let Block::Paragraph { content } = &graph.blocks[2].block
    else {
        panic!("a self-closing SVG child must not swallow the following block, got {:?}", graph.blocks)
    };
    assert!(content.iter().map(Inline::plain_text).collect::<String>().contains("after"));

    let output = export_html(&graph).expect("export html");
    assert!(output.contains(markup), "{output}");
}

#[test]
fn html_import_keeps_non_diagram_classes_on_the_default_path() {
    let lookalike = import_html_text("mermaid-lookalike.html", "<body><div class=\"mermaid-theme\"><p>not a diagram</p></div></body>")
        .expect("import lookalike");
    assert!(single_diagram_opt(&lookalike).is_none(), "substring classes must not match: {:?}", lookalike.blocks);

    let rust = import_html_text("rust-code.html", "<body><pre><code class=\"language-rust\">fn main() {}</code></pre></body>")
        .expect("import rust code");
    let Block::Code { language, content } = &rust.blocks[0].block
    else {
        panic!("plain code block expected, got {:?}", rust.blocks)
    };
    assert_eq!(language.as_deref(), Some("language-rust"));
    assert_eq!(content, "fn main() {}");
}

fn single_diagram_opt(graph: &DocumentGraph) -> Option<(String, String)> {
    graph.blocks.iter().find_map(|node| match &node.block {
        Block::Diagram { language, source } => Some((language.clone(), source.clone())),
        _ => None,
    })
}

#[test]
fn html_mermaid_containers_reopen_through_markdown_as_diagrams() {
    use document_formats::{export::markdown::export_markdown, import::markdown::import_markdown_bytes};

    let sequence = "sequenceDiagram\n  Alice->>Bob: Hi";
    let graph =
        import_html_text("mermaid-roundtrip.html", &format!("<body><div class=\"mermaid\">\n{sequence}\n</div></body>")).expect("import html");
    assert_eq!(single_diagram(&graph).0, "mermaid");

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("```mermaid"), "{markdown}");
    let reopened = import_markdown_bytes("mermaid-roundtrip.md", &markdown).expect("reopen markdown");
    assert_eq!(single_diagram(&reopened), ("mermaid".to_string(), sequence.to_string()));
}

#[test]
fn html_import_records_block_source_spans() {
    let html = "<body><h1>Title</h1><p>Hello</p></body>";
    let graph = import_html_text("spans.html", html).expect("import html");
    assert_eq!(graph.blocks.len(), 2);
    assert_eq!(graph.sources.len(), 2);
    for source in &graph.sources {
        let SourceKind::TextSpan { start, end } = source.kind
        else {
            panic!("text span expected, got {:?}", source.kind)
        };
        assert!(start < end, "{source:?}");
        assert!(end as usize <= html.len(), "{source:?}");
        assert_eq!(source.precision, SemanticStatus::Resolved);
    }
}

#[test]
fn html_import_keeps_the_head_title_out_of_the_body() {
    // A document without `<body>` falls back to lowering its root nodes. The
    // `<head>` subtree carries metadata only, so `<title>` must not reopen as
    // a body paragraph on the way through `html -> html -> markdown`.
    for source in [
        r#"<html><head><title>Doc Title</title></head><p>Body text.</p></html>"#,
        r#"<title>Doc Title</title><p>Body text.</p>"#,
        r#"<head><title>Doc Title</title></head><p>Body text.</p>"#,
    ] {
        let graph = import_html_text("title.html", source).expect("import html");
        assert_eq!(graph.metadata.title.as_deref(), Some("Doc Title"), "{source}");
        let html = export_html(&graph).expect("export html");
        let reopened = import_html_text("reopen.html", &html).expect("reopen html");
        assert_eq!(reopened.metadata.title.as_deref(), Some("Doc Title"), "{html}");
        assert_eq!(reopened.blocks.len(), 1, "the title must not reopen as body content: {html}");
        let Block::Paragraph { content } = &reopened.blocks[0].block
        else {
            panic!("expected the body paragraph, got {:?}", reopened.blocks)
        };
        assert!(matches!(content.first(), Some(Inline::Text { text }) if text == "Body text."), "{content:?}");
    }
}

#[test]
fn html_import_keeps_inline_content_under_containers() {
    let graph = import_html_text(
        "container.html",
        r#"<body><div>Loose text <img src="a.png" alt="A"> tail</div><figure><img src="b.png" alt="B"></figure></body>"#,
    )
    .expect("import html");
    let paragraphs = graph.blocks.iter().filter(|node| matches!(node.block, Block::Paragraph { .. })).count();
    assert_eq!(paragraphs, 2, "{:?}", graph.blocks);
    assert_eq!(graph.assets.len(), 2);
}

#[test]
fn html_import_decodes_character_references_and_keeps_word_boundaries() {
    let graph = import_html_text(
        "entities.html",
        "<html><head><title>Case &amp; Title</title></head><body><p>hello <strong>world</strong> again</p></body></html>",
    )
    .expect("import html");
    assert_eq!(graph.metadata.title.as_deref(), Some("Case & Title"));
    let Some(node) = graph.blocks.iter().find(|node| matches!(node.block, Block::Paragraph { .. }))
    else {
        panic!("expected paragraph, got {:?}", graph.blocks)
    };
    let Block::Paragraph { content } = &node.block
    else {
        unreachable!()
    };
    assert!(matches!(content.first(), Some(Inline::Text { text }) if text == "hello "), "{content:?}");
    assert!(matches!(content.last(), Some(Inline::Text { text }) if text == " again"), "{content:?}");
}

#[test]
fn html_import_reports_link_candidates_with_base_and_source_spans() {
    let html = r#"<html><head><base href="/docs/"><script src="/skip.js"></script></head><body><a href="a.html">A</a><img src="i.png" srcset="s1.png 1x, s2.png 2x"><form action="/submit"></form></body></html>"#;
    let import = import_html_with_links("links.html", html).expect("import html");
    assert_eq!(import.base_href.as_deref(), Some("/docs/"));
    let roles = import.links.iter().map(|link| (link.target.as_str(), link.role)).collect::<Vec<_>>();
    assert_eq!(
        roles,
        vec![
            ("a.html", HtmlLinkRole::Href),
            ("i.png", HtmlLinkRole::Src),
            ("s1.png", HtmlLinkRole::SrcSet),
            ("s2.png", HtmlLinkRole::SrcSet),
            ("/submit", HtmlLinkRole::FormAction),
        ]
    );
    for link in &import.links {
        assert!(link.start < link.end, "{link:?}");
        assert!(link.end <= html.len(), "{link:?}");
    }
}

#[test]
fn html_import_keeps_link_targets_verbatim() {
    let import = import_html_with_links("verbatim.html", r#"<body><a href=" ../a?x=1#f ">A</a></body>"#).expect("import html");
    assert_eq!(import.links[0].target, "../a?x=1#f");
}

#[test]
fn html_import_lowers_common_blocks_and_reports_partial_surface() {
    let graph = import_html_text("sample.html", r#"<html><head><title>Document title</title></head><body><h1>Title</h1><p>Hello <strong>world</strong> <a href="https://example.com">link</a>.</p><ul><li>one</li></ul></body></html>"#).expect("import html");
    assert_eq!(graph.metadata.title.as_deref(), Some("Document title"));
    assert!(matches!(graph.blocks[0].block, Block::Section { .. }));
    assert!(
        matches!(&graph.blocks[1].block, Block::Paragraph { content } if content.iter().any(|inline| matches!(inline, Inline::Styled { style, .. } if style == "bold")))
    );
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.html.partial_coverage" && loss.status == SemanticStatus::Partial));
}

#[test]
fn html_import_keeps_interactive_markup_as_raw_html() {
    let graph = import_html_text("interactive.html", "<body><script>alert(1)</script><form><input></form></body>").expect("import html");
    for code in ["import.html.raw_script", "import.html.raw_form", "import.html.raw_form_control"] {
        assert!(graph.coverage.loss.iter().any(|loss| loss.code == code), "missing {code}: {:?}", graph.coverage.loss);
    }
    let raw: Vec<&str> = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::RawHtml { html } => Some(html.as_str()),
            _ => None,
        })
        .collect();
    assert!(raw.iter().any(|html| html.contains("<script>alert(1)</script>")), "{raw:?}");
    assert!(raw.iter().any(|html| html.contains("<form><input></form>")), "{raw:?}");
}

#[test]
fn html_import_does_not_drop_sibling_blocks_or_nested_list_content() {
    let graph =
        import_html_text("nested.html", r#"<body><div><p>first</p><p>second</p><ul><li>outer<ul><li>inner</li></ul></li></ul></div></body>"#)
            .expect("import html");
    assert_eq!(graph.blocks.iter().filter(|node| matches!(node.block, Block::Paragraph { .. })).count(), 2);
    let list = graph
        .blocks
        .iter()
        .find(|node| matches!(&node.block, Block::List { items, .. } if items.iter().any(|item| item.content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("outer"))))))
        .expect("outer list");
    let Block::List { items, .. } = &list.block
    else {
        unreachable!()
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].children.len(), 1);
    let child = graph.block(items[0].children[0]).expect("nested list node");
    assert!(matches!(child.block, Block::List { .. }));
}

#[test]
fn html_import_numbers_ordered_list_items_and_keeps_unordered_items_bare() {
    let graph = import_html_text(
        "ordered.html",
        r#"<body><ol><li>first</li><li>second</li></ol><ol start="5"><li>shifted</li></ol><ul><li>bare</li></ul></body>"#,
    )
    .expect("import html");
    let lists: Vec<&Vec<document_ir::ListItem>> = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::List { items, .. } => Some(items),
            _ => None,
        })
        .collect();
    assert_eq!(lists.len(), 3, "two ordered lists and one unordered list");
    assert_eq!(lists[0].iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
    assert_eq!(lists[1].iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(5)], "`start` shifts the sequence");
    assert_eq!(lists[2].iter().map(|item| item.number).collect::<Vec<_>>(), vec![None], "unordered items stay unnumbered");
}

#[test]
fn html_import_lowers_breaks_to_a_first_class_inline() {
    let graph = import_html_text("break.html", "<body><p>first<br>second</p></body>").expect("import html");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(content[1], Inline::Break { kind: BreakKind::Hard }), "{content:?}");
    let output = export_html(&graph).expect("export html");
    assert!(output.contains("first<br>second"), "{output}");
}

#[test]
fn html_import_preserves_authored_inline_identity_and_classes() {
    let graph =
        import_html_text("attributed.html", r#"<body><p><span id="anchor" class="lead note">Marked</span></p></body>"#).expect("import html");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let Inline::Attributed { attributes, children } = &content[0]
    else {
        panic!("attributed inline expected, got {content:?}")
    };
    assert_eq!(attributes.id.as_deref(), Some("anchor"));
    assert_eq!(attributes.classes, ["lead", "note"]);
    assert!(attributes.pairs.is_empty());
    assert_eq!(children, &[Inline::Text { text: "Marked".into() }]);
    let output = export_html(&graph).expect("export html");
    assert!(output.contains(r#"<span id="anchor" class="lead note">Marked</span>"#), "{output}");
}

#[test]
fn html_import_records_dropped_inline_style_declarations() {
    let source = r#"<body><p><span style="color: red">red</span> tail</p></body>"#;
    let graph = import_html_text("inline-style.html", source).expect("import html");
    let (index, loss) = graph
        .coverage
        .loss
        .iter()
        .enumerate()
        .find(|(_, loss)| loss.code == "import.html.inline_style_unsupported")
        .expect("inline CSS must be recorded as a loss");
    assert_eq!(loss.status, SemanticStatus::Unsupported);
    let span = graph.loss_span(index).expect("the loss keeps the authored span");
    assert!(source.get(span.start as usize..span.end as usize).is_some(), "the recorded span must point inside the source: {span:?}");

    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "red tail");
    assert!(
        !content.iter().any(|inline| matches!(inline, Inline::Styled { .. } | Inline::Attributed { .. })),
        "a style declaration must not be invented as a semantic style: {content:?}"
    );
}

#[test]
fn html_import_maps_inline_styles_to_canonical_ir_names() {
    let graph = import_html_text("styles.html", "<body><p><s>gone</s> <del>old</del> <u>under</u> <sup>up</sup> <sub>down</sub></p></body>")
        .expect("import html");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let styles = content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Styled { style, children } => Some((style.as_str(), children.iter().map(Inline::plain_text).collect::<String>())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        styles,
        vec![
            ("strike", "gone".to_string()),
            ("strike", "old".to_string()),
            ("underline", "under".to_string()),
            ("superscript", "up".to_string()),
            ("subscript", "down".to_string()),
        ]
    );

    let output = export_html(&graph).expect("export html");
    for expected in ["<s>gone</s>", "<s>old</s>", "<u>under</u>", "<sup>up</sup>", "<sub>down</sub>"] {
        assert!(output.contains(expected), "{expected}: {output}");
    }
}

#[test]
fn html_import_registers_unresolved_image_and_reopens_export() {
    let graph = import_html_text("image.html", r#"<body><p><img src="assets/picture.png" alt="Picture"></p></body>"#).expect("import html");
    assert_eq!(graph.assets.len(), 1);
    assert_eq!(graph.relations[0].kind, RelationKind::Embeds);
    let output = export_html(&graph).expect("export html");
    assert!(output.contains(r#"<img alt="Picture" src="assets/picture.png">"#));
}

#[test]
fn html_footnote_reference_and_endnotes_round_trip_through_markdown() {
    use document_formats::{export::markdown::export_markdown, import::markdown::import_markdown_bytes};

    let source = "See[^1] after\n\n[^1]: Body.\n";
    let graph = import_markdown_bytes("footnote.md", source).expect("import markdown");
    let html = export_html(&graph).expect("export html");

    assert!(html.contains(r#"<section class="footnotes" role="doc-endnotes">"#), "{html}");
    assert!(html.contains(r#"data-footnote-label="1""#), "{html}");
    assert!(!html.contains("[^1]:"), "the authored GFM payload must not leak as prose: {html}");

    let reopened = import_html_text("footnote.html", &html).expect("import html");
    let markdown = export_markdown(&reopened).expect("export markdown");
    assert_eq!(markdown.trim_end(), source.trim_end(), "footnote round trip must preserve the authored definition byte for byte");

    let rebound = import_markdown_bytes("footnote-reopen.md", &markdown).expect("reopen markdown");
    assert!(matches!(
        &rebound.blocks[0].block,
        Block::Paragraph { content } if content.iter().any(|inline| matches!(inline, Inline::Note { label, .. } if label == "1"))
    ));
    assert!(rebound.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Opaque { kind, payload_hint, .. } if kind == "footnote_definition" && payload_hint.trim() == "[^1]: Body."
    )));
}

#[test]
fn html_import_reads_pandoc_style_footnotes() {
    let html = r##"<body><p>Text<a href="#fn1" class="footnote-ref" id="fnref1" role="doc-noteref"><sup>1</sup></a></p><section class="footnotes" role="doc-endnotes"><ol><li id="fn1"><p>Body.</p></li></ol></section></body>"##;
    let graph = import_html_text("pandoc.html", html).expect("import pandoc footnotes");

    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected, got {:?}", graph.blocks)
    };
    assert!(content.iter().any(|inline| matches!(inline, Inline::Note { label, display: None } if label == "1")), "{content:?}");
    assert!(graph.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Opaque { kind, payload_hint, .. } if kind == "footnote_definition" && payload_hint == "[^1]: Body."
    )));
}

#[test]
fn html_import_flattens_table_cell_spans_into_a_pipe_table() {
    let graph = import_html_text(
        "spans.html",
        "<body><table><tr><th colspan=\"2\">Head</th></tr><tr><td rowspan=\"2\">a</td><td>b</td></tr><tr><td>c</td></tr></table></body>",
    )
    .expect("import html");
    let (_, head, body, _) = table(&graph);
    let rows: Vec<&document_ir::TableRow> = head.iter().chain(body).collect();
    // The covered slots become empty continuation cells and every span
    // collapses to 1, so the table stays a rectangular pipe table.
    assert!(rows.iter().flat_map(|row| &row.cells).all(|cell| cell.colspan == 1 && cell.rowspan == 1));
    assert_eq!(head.len(), 1);
    assert_eq!(body.len(), 2);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(cell_text(&graph, &rows[0].cells[0]), "Head");
    assert!(rows[0].cells[1].children.is_empty());
    assert_eq!(cell_text(&graph, &rows[1].cells[0]), "a");
    assert_eq!(cell_text(&graph, &rows[1].cells[1]), "b");
    assert!(rows[2].cells[0].children.is_empty());
    assert_eq!(cell_text(&graph, &rows[2].cells[1]), "c");
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.html.table_spans" && loss.status == SemanticStatus::Partial),
        "the flatten downgrade must be reported: {:?}",
        graph.coverage.loss
    );
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.html.invalid_cell_span"), "{:?}", graph.coverage.loss);
    assert!(graph.validate().is_valid());
}

#[test]
fn html_import_refuses_invalid_cell_span_instead_of_collapsing_it() {
    let graph = import_html_text("bad-span.html", "<body><table><tr><td colspan=\"0\">a</td><td rowspan=\"wide\">b</td></tr></table></body>")
        .expect("import html");
    let (_, head, body, _) = table(&graph);
    let rows: Vec<&document_ir::TableRow> = head.iter().chain(body).collect();
    assert_eq!(rows[0].cells[0].colspan, 1);
    assert_eq!(rows[0].cells[1].rowspan, 1);
    let invalid: Vec<&document_ir::LossMarker> =
        graph.coverage.loss.iter().filter(|loss| loss.code == "import.html.invalid_cell_span").collect();
    assert_eq!(invalid.len(), 2, "{:?}", graph.coverage.loss);
    assert!(invalid.iter().all(|loss| loss.status == SemanticStatus::Partial));
}

#[test]
fn html_import_lowers_definition_list_to_paragraphs_with_a_loss_code() {
    let graph = import_html_text("definition-list.html", "<body><dl><dt>Term</dt><dd>Definition</dd></dl></body>").expect("import html");
    let texts = graph
        .blocks
        .iter()
        .map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<String>(),
            other => panic!("paragraph expected, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, vec!["Term".to_string(), "Definition".to_string()]);
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.html.definition_list" && loss.status == SemanticStatus::Partial),
        "{:?}",
        graph.coverage.loss
    );
    assert!(graph.validate().is_valid());
}

#[test]
fn nested_table_rows_do_not_leak_into_the_outer_table() {
    let graph = import_html_text(
        "nested-table.html",
        concat!(
            "<body><table><tr><th>Head</th></tr>",
            "<tr><td>outer<table><tr><td>inner-one</td></tr><tr><td>inner-two</td></tr></table></td></tr>",
            "</table></body>"
        ),
    )
    .expect("import html");
    let outer = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } if head.len() == 1 && body.len() == 1 => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .expect("outer table");
    assert_eq!(outer.0.len() + outer.1.len(), 2, "inner rows must not become outer rows");
    assert_eq!(outer.1.len(), 1);
    assert_eq!(outer.1[0].cells.len(), 1);
    let cell = &outer.1[0].cells[0];
    assert_eq!(cell.children.len(), 2, "cell holds its text paragraph and the nested table: {cell:?}");
    assert_eq!(cell_text(&graph, cell), "outer");
    let nested = graph.block(cell.children[1]).expect("nested table node");
    let Block::Table { body: nested_body, .. } = &nested.block
    else {
        panic!("nested table expected, got {:?}", nested.block);
    };
    assert_eq!(nested_body.len(), 2);
    assert!(
        !graph.coverage.loss.iter().any(|loss| loss.code == "import.html.table_nested"),
        "the nested table is preserved, not flattened: {:?}",
        graph.coverage.loss
    );

    let html = export_html(&graph).expect("export html");
    assert_eq!(html.matches("<table").count(), 2, "{html}");
    assert_eq!(html.matches("<tr").count(), 4, "{html}");
    assert_eq!(html.matches("inner-one").count(), 1, "nested content must be copied once, not twice: {html}");
}

#[test]
fn html_import_preserves_table_cell_blocks() {
    let graph =
        import_html_text("cell-blocks.html", "<body><table><tr><th>Head</th></tr><tr><td><p>one</p><p>two</p></td></tr></table></body>")
            .expect("import html");
    let (_, _, body, _) = table(&graph);
    let cell = &body[0].cells[0];
    assert_eq!(cell.children.len(), 2, "both authored paragraphs are cell blocks: {cell:?}");
    assert_eq!(cell_text(&graph, cell), "onetwo");
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.html.table_cell_blocks"), "{:?}", graph.coverage.loss);
}
