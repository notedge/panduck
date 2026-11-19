use document_formats::{FormatDirection, FormatStatus, capabilities, import::tex::import_tex_bytes};
use document_ir::{Block, DocumentGraph, Inline, RelationEndpoint, RelationKind};

fn blocks(graph: &DocumentGraph) -> &[document_ir::BlockNode] {
    &graph.blocks
}

fn paragraph_text(graph: &DocumentGraph) -> Vec<String> {
    blocks(graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(content.iter().map(Inline::plain_text).collect()),
            _ => None,
        })
        .collect()
}

#[test]
fn capability_reports_tex_import_as_partial() {
    let capability = capabilities().into_iter().find(|capability| capability.id == "tex").expect("tex capability");
    assert_eq!(capability.direction, FormatDirection::Import);
    assert_eq!(capability.status, FormatStatus::Partial);
}

#[test]
fn sections_keep_hierarchy() {
    let text = "\\section{One}\nBody.\n\n\\subsection{Two}\n\\subsubsection{Three}\n";
    let graph = import_tex_bytes("sections.tex", text).unwrap();
    let levels: Vec<u8> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Section { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, vec![1, 2, 3]);
    let titles: Vec<String> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Section { title, .. } => Some(title.iter().map(Inline::plain_text).collect()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, vec!["1 One".to_string(), "1.1 Two".to_string(), "1.1.1 Three".to_string()]);
    assert!(paragraph_text(&graph).iter().any(|text| text == "Body."));
}

#[test]
fn styled_inlines_map_to_style_semantics() {
    let graph = import_tex_bytes("styles.tex", "A \\textbf{bold} and \\emph{soft} word.\n").unwrap();
    let content = match &blocks(&graph)[0].block {
        Block::Paragraph { content } => content,
        other => panic!("expected paragraph, got {other:?}"),
    };
    let styles: Vec<(&str, String)> = content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Styled { style, children } => Some((style.as_str(), children.iter().map(Inline::plain_text).collect())),
            _ => None,
        })
        .collect();
    assert_eq!(styles, vec![("bold", "bold".to_string()), ("italic", "soft".to_string())]);
}

#[test]
fn math_keeps_authored_source() {
    let text = "Inline $x^2 + y_1$ math.\n\n$$\\int_0^1 x\\,dx$$\n";
    let graph = import_tex_bytes("math.tex", text).unwrap();
    let inline_math: Vec<String> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
                Inline::InlineMath { formula } => {
                    assert_eq!(formula.language.as_deref(), Some("latex"));
                    Some(formula.source.clone())
                }
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(inline_math, vec!["x^2 + y_1".to_string()]);
    let display_math: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Math { formula } => {
                assert_eq!(formula.language.as_deref(), Some("latex"));
                Some(formula.source.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(display_math.len(), 1);
    assert!(display_math[0].contains("\\int_0^1 x"));
}

#[test]
fn unknown_command_stays_opaque_and_records_loss() {
    let graph = import_tex_bytes("unknown.tex", "\\foo{bar}\n").unwrap();
    let opaque: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Opaque { kind, payload_hint, .. } => {
                assert!(payload_hint.contains("\\foo"));
                Some(kind.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(opaque, vec!["tex.foo"]);
    assert!(!graph.coverage.complete);
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.tex.unsupported_command"));
}

#[test]
fn unknown_environment_stays_opaque() {
    let graph = import_tex_bytes("unknown_env.tex", "\\begin{mystery}payload\\end{mystery}\n").unwrap();
    assert!(blocks(&graph).iter().any(|node| matches!(&node.block, Block::Opaque { kind, .. } if kind == "tex.environment.mystery")));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.tex.unknown_environment"));
}

#[test]
fn unordered_and_ordered_lists_lower_with_numbers() {
    let text =
        "\\begin{itemize}\n\\item One\n\\item Two\n\\end{itemize}\n\n\\begin{enumerate}\n\\item First\n\\item Second\n\\end{enumerate}\n";
    let graph = import_tex_bytes("lists.tex", text).unwrap();
    let lists: Vec<&Block> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::List { .. } => Some(&node.block),
            _ => None,
        })
        .collect();
    assert_eq!(lists.len(), 2);
    let Block::List { ordered: false, items } = lists[0]
    else {
        panic!("expected unordered list, got {:?}", lists[0]);
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content.iter().map(Inline::plain_text).collect::<String>(), "One");
    assert_eq!(items[1].content.iter().map(Inline::plain_text).collect::<String>(), "Two");
    assert!(items.iter().all(|item| item.number.is_none()));

    let Block::List { ordered: true, items } = lists[1]
    else {
        panic!("expected ordered list, got {:?}", lists[1]);
    };
    assert_eq!(items.iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
}

#[test]
fn nested_list_becomes_list_item_child() {
    let text = "\\begin{itemize}\n\\item Outer\n  \\begin{enumerate}\n  \\item Inner\n  \\end{enumerate}\n\\end{itemize}\n";
    let graph = import_tex_bytes("nested.tex", text).unwrap();
    let outer = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::List { ordered: false, items } => Some(items),
            _ => None,
        })
        .expect("outer unordered list");
    assert_eq!(outer.len(), 1);
    assert_eq!(outer[0].children.len(), 1);
    let nested = graph.block(outer[0].children[0]).expect("nested list");
    let Block::List { ordered: true, items: nested_items } = &nested.block
    else {
        panic!("expected nested list, got {:?}", nested.block);
    };
    assert_eq!(nested_items.len(), 1);
    assert_eq!(nested_items[0].content.iter().map(Inline::plain_text).collect::<String>(), "Inner");
}

#[test]
fn simple_tabular_becomes_table() {
    let text = "\\begin{tabular}{lc}\nName & Value \\\\\nA & 1 \\\\\n\\end{tabular}\n";
    let graph = import_tex_bytes("table.tex", text).unwrap();
    let table = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { body, .. } => Some(body),
            _ => None,
        })
        .expect("table block");
    let cell_text = |cell: &document_ir::TableCell| {
        cell.children
            .iter()
            .filter_map(|id| graph.block(*id))
            .flat_map(|node| match &node.block {
                Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect::<String>()
    };
    assert_eq!(table.len(), 2);
    assert_eq!(table[0].cells.len(), 2);
    assert_eq!(cell_text(&table[0].cells[0]), "Name");
    assert_eq!(cell_text(&table[1].cells[1]), "1");
}

#[test]
fn verbatim_and_verb_become_code() {
    let text = "\\begin{verbatim}\nlet x = 1;\n\\end{verbatim}\n\n\\verb|inline|\n";
    let graph = import_tex_bytes("code.tex", text).unwrap();
    let code_blocks: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Code { content, .. } => Some(content.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(code_blocks, vec!["let x = 1;"]);
    // `\verb` is text-mode inline material, so it stays `InlineCode` inside the
    // surrounding paragraph instead of becoming a block.
    let inline_code: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(content),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::InlineCode { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(inline_code, vec!["inline"]);
}

#[test]
fn includegraphics_becomes_image_and_registers_asset() {
    let graph = import_tex_bytes("image.tex", "\\includegraphics[width=3cm]{figure.png}\n").unwrap();
    let images: Vec<&Inline> = blocks(&graph)
        .iter()
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .filter(|inline| matches!(inline, Inline::Image { .. }))
        .collect();
    assert_eq!(images.len(), 1);
    let Inline::Image { source, asset, .. } = images[0]
    else {
        unreachable!()
    };
    assert_eq!(source, "figure.png");
    assert!(asset.is_some());
    assert_eq!(graph.assets.len(), 1);
}

#[test]
fn left_right_pairs_keep_authored_delimiters() {
    let text = "\\left( x + y \\right) \\left[ a \\right] \\left\\{ b \\right\\}\n";
    let graph = import_tex_bytes("delimiters.tex", text).unwrap();
    let pairs: Vec<(String, String, String)> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(delimiter_pair)
        .map(|(left, right, children)| (left.to_string(), right.to_string(), children.iter().map(Inline::plain_text).collect()))
        .collect();
    assert_eq!(
        pairs,
        vec![
            ("(".to_string(), ")".to_string(), "x+y".to_string()),
            ("[".to_string(), "]".to_string(), "a".to_string()),
            ("\\{".to_string(), "\\}".to_string(), "b".to_string()),
        ]
    );
}

#[test]
fn nested_left_right_pairs_keep_inner_structure() {
    let graph = import_tex_bytes("nested_delimiters.tex", "\\left( \\left[ x \\right] \\right)\n").unwrap();
    let inlines = paragraph_inlines(&graph);
    let outer = inlines.iter().find_map(|inline| delimiter_pair(inline)).expect("outer delimiter pair");
    assert_eq!((outer.0, outer.1), ("(", ")"));
    let inner = outer.2.iter().find_map(delimiter_pair).expect("inner delimiter pair");
    assert_eq!((inner.0, inner.1), ("[", "]"));
    assert_eq!(inner.2.iter().map(Inline::plain_text).collect::<String>(), "x");
    // The inner pair stays its own node instead of flattening into the outer body
    // text.
    assert!(matches!(outer.2, [Inline::Attributed { .. }]));
}

#[test]
fn limits_marker_stays_adjacent_to_its_operator() {
    let graph = import_tex_bytes("limits.tex", "\\sum\\limits_{i=1}^{n}\n").unwrap();
    assert!(
        blocks(&graph)
            .iter()
            .any(|node| matches!(&node.block, Block::Opaque { kind, payload_hint, .. } if kind == "tex.sum" && payload_hint == "\\sum"))
    );

    let inlines = paragraph_inlines(&graph);
    let marker = inlines.first().expect("limits marker inline");
    assert_eq!(limits_value(marker), Some("limits"));
    assert!(matches!(marker, Inline::Attributed { children, .. } if children.is_empty()));

    let scripts: Vec<(&str, String)> = inlines[1..]
        .iter()
        .filter_map(|inline| match inline {
            Inline::Styled { style, children } => Some((style.as_str(), children.iter().map(Inline::plain_text).collect())),
            _ => None,
        })
        .collect();
    assert_eq!(scripts, vec![("subscript", "i=1".to_string()), ("superscript", "n".to_string())]);
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.tex.limits_marker"));
}

#[test]
fn nolimits_marker_records_the_nolimits_variant() {
    let graph = import_tex_bytes("nolimits.tex", "\\int\\nolimits_0^1\n").unwrap();
    assert!(
        blocks(&graph)
            .iter()
            .any(|node| matches!(&node.block, Block::Opaque { kind, payload_hint, .. } if kind == "tex.int" && payload_hint == "\\int"))
    );

    let inlines = paragraph_inlines(&graph);
    let marker = inlines.first().expect("nolimits marker inline");
    assert_eq!(limits_value(marker), Some("nolimits"));

    let scripts: Vec<(&str, String)> = inlines[1..]
        .iter()
        .filter_map(|inline| match inline {
            Inline::Styled { style, children } => Some((style.as_str(), children.iter().map(Inline::plain_text).collect())),
            _ => None,
        })
        .collect();
    assert_eq!(scripts, vec![("subscript", "0".to_string()), ("superscript", "1".to_string())]);
}

#[test]
fn math_delimiters_share_inline_and_display_semantics() {
    let text = "Inline $x$ and \\(y\\) math.\n\n$$a$$\n\n\\[b\\]\n";
    let graph = import_tex_bytes("delimiters_eq.tex", text).unwrap();
    let inline_math: Vec<String> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::InlineMath { formula } => {
                assert_eq!(formula.language.as_deref(), Some("latex"));
                Some(formula.source.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(inline_math, vec!["x".to_string(), "y".to_string()]);

    let display_math: Vec<String> = math_blocks(&graph).into_iter().map(|(content, _, _)| content).collect();
    assert_eq!(display_math, vec!["a".to_string(), "b".to_string()]);
    assert!(!has_loss(&graph, "import.tex.unclosed_math"));
}

#[test]
fn math_environments_lower_to_math_blocks_with_source_spans() {
    let text = "\\begin{aligned}\na &= b \\\\\nc &= d\n\\end{aligned}\n\n\\begin{cases}1 & x>0 \\\\ 0 & x\\le 0\\end{cases}\n";
    let graph = import_tex_bytes("environments.tex", text).unwrap();
    let maths = math_blocks(&graph);
    assert_eq!(maths.len(), 2, "each math environment becomes one math block");
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "math environments must not stay opaque");

    let (aligned, aligned_id, _) = &maths[0];
    assert!(aligned.starts_with("\\begin{aligned}"));
    assert!(aligned.ends_with("\\end{aligned}"));
    let (cases, _, _) = &maths[1];
    assert!(cases.starts_with("\\begin{cases}"));
    assert_eq!(cases.matches('&').count(), 2, "cases keeps one alignment tab per row");
    assert_eq!(cases.matches("\\\\").count(), 1, "cases keeps its row break");

    for (content, node, _) in &maths {
        let (start, end) = block_span(&graph, *node).expect("math block source span");
        assert_eq!(&text[start as usize..end as usize], content.as_str());
    }
    let (aligned_start, aligned_end) = block_span(&graph, *aligned_id).expect("aligned span");
    assert!(aligned_end > aligned_start);
    assert_eq!(graph.sources_for(*aligned_id).len(), 1, "one span per math block");
    assert!(!has_loss(&graph, "import.tex.math_numbering"), "inner environments are never numbered");
}

#[test]
fn numbered_math_environments_record_the_missing_number_anchor() {
    let text = "\\begin{align}a&=b\\\\c&=d\\end{align}\n\n\\begin{align*}a&=b\\end{align*}\n\n\\begin{equation}x=1\\end{equation}\n\n\\begin{equation*}y=2\\end{equation*}\n";
    let graph = import_tex_bytes("numbering.tex", text).unwrap();
    let maths = math_blocks(&graph);
    let opened: Vec<String> = maths.iter().map(|(content, _, _)| content.lines().next().unwrap_or("").to_string()).collect();
    assert_eq!(
        opened,
        vec![
            "\\begin{align}a&=b\\\\c&=d\\end{align}".to_string(),
            "\\begin{align*}a&=b\\end{align*}".to_string(),
            "\\begin{equation}x=1\\end{equation}".to_string(),
            "\\begin{equation*}y=2\\end{equation*}".to_string()
        ]
    );

    let numbering: Vec<&str> =
        graph.coverage.loss.iter().filter(|loss| loss.code == "import.tex.math_numbering").map(|loss| loss.message.as_str()).collect();
    assert_eq!(numbering.len(), 2, "only the unstarred equation and align carry a number");
    assert!(numbering.iter().any(|message| message.contains("align`")));
    assert!(numbering.iter().any(|message| message.contains("equation`")));
    assert!(!numbering.iter().any(|message| message.contains('*')));
    assert!(!has_loss(&graph, "import.tex.unknown_environment"));
}

#[test]
fn math_environment_inside_document_keeps_block_order_and_span() {
    let text = "\\begin{document}\n\\section{One}\n\\begin{gathered}a \\\\ b\\end{gathered}\n\\end{document}\n";
    let graph = import_tex_bytes("document_math.tex", text).unwrap();
    let kinds: Vec<&str> = blocks(&graph)
        .iter()
        .map(|node| match &node.block {
            Block::Section { .. } => "section",
            Block::Math { .. } => "math",
            Block::Paragraph { .. } => "paragraph",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, vec!["section", "math"]);
    let (content, node, _) = &math_blocks(&graph)[0];
    assert!(content.starts_with("\\begin{gathered}"));
    let (start, end) = block_span(&graph, *node).expect("nested math source span");
    assert_eq!(&text[start as usize..end as usize], content.as_str());
}

#[test]
fn math_environment_in_inline_context_stays_inline_math() {
    let graph = import_tex_bytes("inline_environment.tex", "Body {\\begin{cases}a&b\\end{cases}} tail.\n").unwrap();
    let inline_math: Vec<String> = paragraph_inlines(&graph)
        .into_iter()
        .flat_map(|inline| match inline {
            Inline::InlineMath { formula, .. } => vec![formula.source.clone()],
            Inline::Styled { children, .. } | Inline::Attributed { children, .. } => children
                .iter()
                .filter_map(|child| match child {
                    Inline::InlineMath { formula, .. } => Some(formula.source.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(inline_math, vec!["\\begin{cases}a&b\\end{cases}".to_string()]);
    assert!(blocks(&graph).iter().all(|node| matches!(&node.block, Block::Paragraph { .. })));
}

#[test]
fn comments_leave_no_text_artifacts() {
    let graph = import_tex_bytes("comments.tex", "a % note to self\nb\n").unwrap();
    assert_eq!(paragraph_text(&graph), vec!["a b".to_string()]);
    assert!(graph.coverage.loss.iter().all(|loss| !loss.code.contains("comment")));
    let graph = import_tex_bytes("gobble.tex", "a% note\nb\n").unwrap();
    assert_eq!(paragraph_text(&graph), vec!["ab".to_string()], "a comment swallows its terminating newline");
}

#[test]
fn paragraph_whitespace_follows_tex_collapsing() {
    let graph = import_tex_bytes("spaces.tex", "hello   world\n\nsecond\n").unwrap();
    assert_eq!(paragraph_text(&graph), vec!["hello world".to_string(), "second".to_string()]);
}

#[test]
fn escaped_commands_become_author_characters() {
    let graph = import_tex_bytes("escapes.tex", "\\$5\\%\\&\\_\n").unwrap();
    assert_eq!(paragraph_text(&graph), vec!["$5%&_".to_string()]);

    let graph = import_tex_bytes("line_break.tex", "left\\\\right\n").unwrap();
    assert_eq!(paragraph_text(&graph), vec!["left\nright".to_string()]);
    let break_kinds: Vec<document_ir::BreakKind> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::Break { kind } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(break_kinds, vec![document_ir::BreakKind::Hard]);
}

#[test]
fn unclosed_math_delimiter_records_loss_and_keeps_the_authored_body() {
    let inline_text = "a $x\n";
    let graph = import_tex_bytes("unclosed_inline.tex", inline_text).unwrap();
    let inline_math: Vec<String> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::InlineMath { formula, .. } => Some(formula.source.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(inline_math, vec!["x".to_string()]);
    assert_eq!(loss_count(&graph, "import.tex.unclosed_math"), 1);

    let display_text = "$$y\n";
    let graph = import_tex_bytes("unclosed_display.tex", display_text).unwrap();
    let (content, node, _) = &math_blocks(&graph)[0];
    assert_eq!(content, "y");
    let (start, end) = block_span(&graph, *node).expect("unclosed display math span");
    assert_eq!(display_text[start as usize..end as usize].trim(), "$$y");
    assert_eq!(loss_count(&graph, "import.tex.unclosed_math"), 1);
}

#[test]
fn mismatched_end_records_loss_and_keeps_the_environment_source() {
    let text = "\\begin{align}a=1\\end{cases}\n";
    let graph = import_tex_bytes("mismatched.tex", text).unwrap();
    let maths = math_blocks(&graph);
    assert_eq!(maths.len(), 1);
    assert_eq!(maths[0].0, "\\begin{align}a=1\\end{cases}");
    let (start, end) = block_span(&graph, maths[0].1).expect("mismatched environment span");
    assert_eq!(&text[start as usize..end as usize], maths[0].0.as_str());
    assert!(has_loss(&graph, "import.tex.mismatched_end"));
    assert!(!has_loss(&graph, "import.tex.unclosed_environment"));
}

#[test]
fn unclosed_left_right_records_loss_and_keeps_the_body() {
    let graph = import_tex_bytes("unclosed_left.tex", "\\left( x\n").unwrap();
    let pair = paragraph_inlines(&graph).into_iter().find_map(delimiter_pair).expect("delimiter pair");
    assert_eq!((pair.0, pair.1), ("(", ""));
    assert_eq!(pair.2.iter().map(Inline::plain_text).collect::<String>(), "x");
    assert!(has_loss(&graph, "import.tex.unclosed_delimiters"));
}

#[test]
fn starred_section_records_the_missing_numbering_state() {
    let graph = import_tex_bytes("starred_section.tex", "\\section*{One}\n").unwrap();
    assert!(blocks(&graph).iter().any(|node| matches!(&node.block, Block::Section { level: 1, .. })));
    assert!(has_loss(&graph, "import.tex.starred_command"));
}

/// Display math blocks with their content, node id, and authored span start.
fn math_blocks(graph: &DocumentGraph) -> Vec<(String, document_ir::NodeId, u32)> {
    blocks(graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Math { formula } => {
                assert_eq!(formula.language.as_deref(), Some("latex"));
                let start = match block_span(graph, node.id) {
                    Some((start, _)) => start,
                    None => 0,
                };
                Some((formula.source.clone(), node.id, start))
            }
            _ => None,
        })
        .collect()
}

fn block_span(graph: &DocumentGraph, node: document_ir::NodeId) -> Option<(u32, u32)> {
    graph.sources_for(node).into_iter().find_map(|source| match &source.kind {
        document_ir::SourceKind::TextSpan { start, end } => Some((*start, *end)),
        _ => None,
    })
}

fn has_loss(graph: &DocumentGraph, code: &str) -> bool {
    graph.coverage.loss.iter().any(|loss| loss.code == code)
}

fn loss_count(graph: &DocumentGraph, code: &str) -> usize {
    graph.coverage.loss.iter().filter(|loss| loss.code == code).count()
}

/// Inline content of every paragraph block in document order.
fn paragraph_inlines(graph: &DocumentGraph) -> Vec<&Inline> {
    blocks(graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(content.iter()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// Authored `\left...\right` delimiters plus the lowered body of the pair.
fn delimiter_pair(inline: &Inline) -> Option<(&str, &str, &[Inline])> {
    let Inline::Attributed { attributes, children } = inline
    else {
        return None;
    };
    if !attributes.classes.iter().any(|class| class == "tex-delimiter-pair") {
        return None;
    }
    let value = |key: &str| attributes.pairs.iter().find(|pair| pair.key == key).map(|pair| pair.value.as_str());
    Some((value("data-tex-left")?, value("data-tex-right")?, children.as_slice()))
}

/// Authored `\limits` / `\nolimits` variant recorded by the marker inline.
fn limits_value(inline: &Inline) -> Option<&str> {
    let Inline::Attributed { attributes, .. } = inline
    else {
        return None;
    };
    if !attributes.classes.iter().any(|class| class == "tex-limits") {
        return None;
    }
    attributes.pairs.iter().find(|pair| pair.key == "data-tex-limits").map(|pair| pair.value.as_str())
}

#[test]
fn math_style_commands_outside_math_source_are_dropped_silently() {
    // These commands belong inside math mode; lowering drops them silently
    // without emitting a loss marker, because the math layout stage (VuTeX)
    // only consumes math spans preserved as Block::Math and never sees these
    // stray non-math tokens.
    let graph = import_tex_bytes(
        "math_style_cmds.tex",
        "Pre formula text.\n\n\\displaystyle\n\\textstyle\n\\scriptstyle\n\\scriptscriptstyle\n\nPost formula text.\n",
    )
    .unwrap();
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "math style commands must not stay opaque");
    assert_eq!(loss_count(&graph, "import.tex.math_style_command"), 0);
}

#[test]
fn math_font_commands_outside_math_source_are_dropped_silently() {
    let graph = import_tex_bytes(
        "math_font_cmds.tex",
        "See \\mathbf{A} and \\mathit{B} and \\mathrm{C} and \\mathcal{D} and \\mathbb{E} and \\mathfrak{F}.\n",
    )
    .unwrap();
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "math font commands must not stay opaque");
    assert_eq!(loss_count(&graph, "import.tex.math_font_command"), 0);
}

#[test]
fn math_font_commands_with_arguments_do_not_stay_opaque() {
    // Math font commands are consumed by the lowering without producing a
    // dangling opaque block, even when they carry an argument.
    let graph = import_tex_bytes("math_font_arg.tex", "Before \\mathbf{abc} after.\n").unwrap();
    assert!(
        blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })),
        "math font commands with arguments must not stay opaque"
    );
    assert_eq!(loss_count(&graph, "import.tex.math_font_command"), 0);
}

#[test]
fn limits_marker_stays_inline_without_extra_loss_code() {
    // `\limits` / `\nolimits` become `TexItem::Limits` → attributed inline plus
    // one `limits_marker` loss. The redundant `math_limits_command` loss is
    // gone because lowering no longer emits math-style loss codes.
    let graph = import_tex_bytes("math_limits_loss.tex", "\\sum\\limits_{i=1}^{n}\n").unwrap();
    assert!(has_loss(&graph, "import.tex.limits_marker"));
    assert_eq!(loss_count(&graph, "import.tex.math_limits_command"), 0);
}

#[test]
fn text_mode_space_commands_keep_their_authored_distinction() {
    let graph = import_tex_bytes("spaces.tex", "a\\,b\\:c\\;d\\!e\\quad f\\qquad g\n").unwrap();
    let spaces: Vec<String> = paragraph_inlines(&graph).into_iter().filter_map(space_value).collect();
    assert_eq!(spaces, vec!["thin", "medium", "thick", "negative", "quad", "double-quad"]);
    assert_eq!(loss_count(&graph, "import.tex.space_marker"), 6, "one loss per authored spacing command");
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "spacing commands must not stay opaque");
    assert!(!has_loss(&graph, "import.tex.unsupported_command"));
}

#[test]
fn math_mode_space_commands_stay_verbatim_in_the_preserved_source() {
    // Inside math the authored commands are part of the preserved math payload,
    // so the layout host can still distinguish 3mu / 4mu / 5mu values and no
    // text-mode spacing marker or loss is emitted.
    let graph = import_tex_bytes("math_spaces.tex", "$a\\,b\\:c\\;d\\!e\\quad f\\qquad g$\n").unwrap();
    let inline_math: Vec<String> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::InlineMath { formula, .. } => Some(formula.source.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(inline_math.len(), 1);
    for command in ["\\,", "\\:", "\\;", "\\!", "\\quad", "\\qquad"] {
        assert!(inline_math[0].contains(command), "math payload must keep `{command}` verbatim: {}", inline_math[0]);
    }
    assert_eq!(loss_count(&graph, "import.tex.space_marker"), 0);
    assert_eq!(paragraph_inlines(&graph).into_iter().filter_map(space_value).count(), 0);
}

/// Authored spacing variant recorded by a `tex-space` marker inline.
fn space_value(inline: &Inline) -> Option<String> {
    let Inline::Attributed { attributes, .. } = inline
    else {
        return None;
    };
    if !attributes.classes.iter().any(|class| class == "tex-space") {
        return None;
    }
    attributes.pairs.iter().find(|pair| pair.key == "data-tex-space").map(|pair| pair.value.clone())
}

#[test]
fn math_commands_outside_math_source_leave_no_loss_or_opaque_blocks() {
    let graph = import_tex_bytes("combined_math_loss.tex", "\\displaystyle \\mathbf{x} \\nolimits_{i} a_{i}\n").unwrap();
    assert_eq!(loss_count(&graph, "import.tex.math_style_command"), 0);
    assert_eq!(loss_count(&graph, "import.tex.math_font_command"), 0);
    assert_eq!(loss_count(&graph, "import.tex.math_limits_command"), 0);
    // No opaque blocks: lowering dropped the style/font commands silently,
    // and limits became an attributed inline via TexItem::Limits.
    assert!(
        blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })),
        "no opaque blocks expected when math commands are recognized"
    );
}

#[test]
fn title_and_author_fill_document_metadata() {
    let text = "\\title{Energy Flow}\n\\author{A. Chen}\n\\author{B. Wu}\n\\date{October 2026}\n\\maketitle\nBody.\n";
    let graph = import_tex_bytes("paper.tex", text).unwrap();
    assert_eq!(graph.metadata.title.as_deref(), Some("Energy Flow"));
    assert_eq!(graph.metadata.authors, vec!["A. Chen".to_string(), "B. Wu".to_string()]);
    assert!(has_loss(&graph, "import.tex.metadata_date"));
    assert!(has_loss(&graph, "import.tex.maketitle_dropped"));
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "metadata commands must not stay opaque");
}

#[test]
fn section_numbers_reset_deeper_counters() {
    let text = "\\section{A}\n\\subsection{B}\n\\section{C}\n";
    let graph = import_tex_bytes("numbers.tex", text).unwrap();
    let titles: Vec<String> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Section { title, .. } => Some(title.iter().map(Inline::plain_text).collect()),
            _ => None,
        })
        .collect();
    assert_eq!(titles, vec!["1 A".to_string(), "1.1 B".to_string(), "2 C".to_string()]);
}

#[test]
fn label_and_ref_resolve_to_section_numbers() {
    let text = "\\section{Intro}\n\\label{sec:intro}\nAs shown in Section~\\ref{sec:intro}.\n";
    let graph = import_tex_bytes("refs.tex", text).unwrap();
    assert!(graph.validate().is_valid());
    let section_node = blocks(&graph).iter().find(|node| matches!(&node.block, Block::Section { .. })).expect("section").id;
    let relations = relation_pairs(&graph, RelationKind::References);
    assert_eq!(relations, vec![("label:sec:intro".to_string(), format!("node:{}", section_node.0))]);
    let refs: Vec<(&str, &str, String, String)> = paragraph_inlines(&graph).into_iter().filter_map(reference_marker).collect();
    assert_eq!(refs.len(), 1);
    assert_eq!((refs[0].0, refs[0].1), ("ref", "sec:intro"));
    assert_eq!(refs[0].2, format!("node:{}", section_node.0));
    assert_eq!(refs[0].3, "1");
    assert!(!has_loss(&graph, "import.tex.unresolved_reference"));
    assert!(!has_loss(&graph, "import.tex.label_anchor"));
}

#[test]
fn eqref_resolves_equation_number_from_label_inside_environment() {
    let text = "\\begin{equation}\\label{eq:energy}E=mc^2\\end{equation}\nSee \\eqref{eq:energy}.\n";
    let graph = import_tex_bytes("eqref.tex", text).unwrap();
    assert!(graph.validate().is_valid());
    let math_node = blocks(&graph).iter().find(|node| matches!(&node.block, Block::Math { .. })).expect("math").id;
    let relations = relation_pairs(&graph, RelationKind::References);
    assert_eq!(relations, vec![("label:eq:energy".to_string(), format!("node:{}", math_node.0))]);
    let refs: Vec<(&str, &str, String, String)> = paragraph_inlines(&graph).into_iter().filter_map(reference_marker).collect();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].0, "eqref");
    assert_eq!(refs[0].2, format!("node:{}", math_node.0));
    assert_eq!(refs[0].3, "(1)");
}

#[test]
fn forward_reference_resolves_in_the_post_pass() {
    let text = "See \\ref{sec:later} first.\n\\section{Later}\n\\label{sec:later}\n";
    let graph = import_tex_bytes("forward.tex", text).unwrap();
    assert!(graph.validate().is_valid());
    let section_node = blocks(&graph).iter().find(|node| matches!(&node.block, Block::Section { .. })).expect("section").id;
    let refs: Vec<(&str, &str, String, String)> = paragraph_inlines(&graph).into_iter().filter_map(reference_marker).collect();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].2, format!("node:{}", section_node.0));
    assert_eq!(refs[0].3, "1");
    assert!(!has_loss(&graph, "import.tex.unresolved_reference"));
}

#[test]
fn unknown_reference_records_loss_and_stays_unresolved() {
    let graph = import_tex_bytes("unknown_ref.tex", "See \\ref{sec:missing}.\n").unwrap();
    let refs: Vec<(&str, &str, String, String)> = paragraph_inlines(&graph).into_iter().filter_map(reference_marker).collect();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].2, "label:sec:missing");
    assert_eq!(refs[0].3, "sec:missing");
    assert_eq!(loss_count(&graph, "import.tex.unresolved_reference"), 1);
    assert!(graph.validate().is_valid());
}

#[test]
fn footnote_becomes_numbered_note_with_lowered_body() {
    let graph = import_tex_bytes("footnote.tex", "Body\\footnote{A \\emph{note}.} tail.\n").unwrap();
    let notes: Vec<(String, String)> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::Note { label, display } => {
                Some((label.clone(), display.as_ref().map(|display| display.iter().map(Inline::plain_text).collect()).unwrap_or_default()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(notes, vec![("1".to_string(), "A note.".to_string())]);
    assert!(has_loss(&graph, "import.tex.footnote_body"));
    assert!(blocks(&graph).iter().all(|node| !matches!(&node.block, Block::Opaque { .. })), "footnote must not stay an opaque block");
    assert!(graph.validate().is_valid());
}

#[test]
fn cite_splits_keys_and_pushes_cites_relations_once_per_key() {
    let text = "Work \\cite{chen2024,wu2025} and \\cite{chen2024}.\n";
    let graph = import_tex_bytes("cite.tex", text).unwrap();
    let keys: Vec<String> = paragraph_inlines(&graph)
        .into_iter()
        .filter_map(|inline| match inline {
            Inline::Citation { key, .. } => Some(key.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(keys, vec!["chen2024".to_string(), "wu2025".to_string(), "chen2024".to_string()]);
    let cites = relation_pairs(&graph, RelationKind::Cites);
    assert_eq!(cites.len(), 2, "one relation per distinct key");
    assert_eq!(cites[0].1, "label:chen2024");
    assert_eq!(cites[1].1, "label:wu2025");
    assert!(cites.iter().all(|(source, _)| source.starts_with("document:")));
    assert!(graph.validate().is_valid());
}

#[test]
fn notag_and_tag_shape_equation_numbering() {
    let text = "\\begin{align}a&=b\\notag\\\\c&=d\\label{eq:kept}\\end{align}\n\\begin{equation}\\tag{7}\\label{eq:tagged}x=1\\end{equation}\nSee \\eqref{eq:kept} and \\eqref{eq:tagged}.\n";
    let graph = import_tex_bytes("notag.tex", text).unwrap();
    assert!(graph.validate().is_valid());
    let refs: Vec<(&str, &str, String, String)> = paragraph_inlines(&graph).into_iter().filter_map(reference_marker).collect();
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].3, "(1)", "the notag row consumes no number");
    assert_eq!(refs[1].3, "(7)", "the tag replaces the displayed number");
}

/// Reference kind, authored label, lowered link target, and display text of a
/// `tex-ref` marker inline.
fn reference_marker(inline: &Inline) -> Option<(&str, &str, String, String)> {
    let Inline::Attributed { attributes, children } = inline
    else {
        return None;
    };
    if !attributes.classes.iter().any(|class| class == "tex-ref") {
        return None;
    }
    let kind = attributes.pairs.iter().find(|pair| pair.key == "data-tex-ref-kind")?.value.as_str();
    let label = attributes.pairs.iter().find(|pair| pair.key == "data-tex-ref-label")?.value.as_str();
    let Inline::Link { target, display, .. } = children.first()?
    else {
        return None;
    };
    Some((kind, label, endpoint_name(target), display.iter().map(Inline::plain_text).collect()))
}

/// Relations of one kind with both endpoints rendered as stable strings.
fn relation_pairs(graph: &DocumentGraph, kind: RelationKind) -> Vec<(String, String)> {
    graph
        .relations
        .iter()
        .filter(|relation| relation.kind == kind)
        .map(|relation| (endpoint_name(&relation.source), endpoint_name(&relation.target)))
        .collect()
}

fn endpoint_name(endpoint: &RelationEndpoint) -> String {
    match endpoint {
        RelationEndpoint::Unresolved { label } => format!("label:{label}"),
        RelationEndpoint::Node(id) => format!("node:{}", id.0),
        RelationEndpoint::Document(id) => format!("document:{}", id.0),
        other => format!("{other:?}"),
    }
}

#[test]
fn shared_tex_corpus_holds_its_lowering_contract() {
    use document_formats::tex_corpus::{block_kind_name, tex_conformance_corpus};
    let cases = tex_conformance_corpus();
    assert!(cases.len() >= 20, "the corpus must cover the joint TeX families");
    let mut ids = std::collections::HashSet::new();
    for case in cases {
        assert!(ids.insert(case.id), "duplicate corpus id {}", case.id);
        let graph = import_tex_bytes(&format!("{}.tex", case.id), case.tex)
            .unwrap_or_else(|error| panic!("corpus {} failed to import: {error}", case.id));
        assert!(graph.validate().is_valid(), "corpus {} must validate", case.id);
        let kinds: Vec<&str> = blocks(&graph).iter().map(|node| block_kind_name(&node.block)).collect();
        assert_eq!(kinds.as_slice(), case.expected_block_kinds, "corpus {} block kinds", case.id);
        let payloads = math_payloads(&graph);
        assert_eq!(payloads, case.expected_math_payloads, "corpus {} math payloads", case.id);
        for code in case.expected_loss_codes {
            assert!(has_loss(&graph, code), "corpus {} must record {code}", case.id);
        }
        for code in case.forbidden_loss_codes {
            assert_eq!(loss_count(&graph, code), 0, "corpus {} must not record {code}", case.id);
        }
        assert_eq!(graph.metadata.title.as_deref(), case.expected_title, "corpus {} title", case.id);
        let authors: Vec<&str> = graph.metadata.authors.iter().map(String::as_str).collect();
        assert_eq!(authors.as_slice(), case.expected_authors, "corpus {} authors", case.id);
    }
}

/// Math payloads kept for the layout stage, in document order.
fn math_payloads(graph: &DocumentGraph) -> Vec<&str> {
    let mut payloads = Vec::new();
    for node in blocks(graph) {
        match &node.block {
            Block::Math { formula, .. } => payloads.push(formula.source.as_str()),
            Block::Paragraph { content } => {
                for inline in content {
                    if let Inline::InlineMath { formula, .. } = inline {
                        payloads.push(formula.source.as_str());
                    }
                }
            }
            _ => {}
        }
    }
    payloads
}
