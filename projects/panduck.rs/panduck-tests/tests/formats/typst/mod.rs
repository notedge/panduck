use document_formats::{FormatDirection, FormatStatus, capabilities, import::typst::import_typst_bytes};
use document_ir::{Block, DocumentGraph, Inline, MathDisplay, MathExpression};

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
fn capability_reports_typst_import_as_partial() {
    let capability = capabilities().into_iter().find(|capability| capability.id == "typst").expect("typst capability");
    assert_eq!(capability.direction, FormatDirection::Import);
    assert_eq!(capability.status, FormatStatus::Partial);
}

#[test]
fn heading_becomes_section_and_tail_keeps_paragraphs() {
    let graph = import_typst_bytes("heading.typ", "= Title\n\nIntro text.\n\nSecond paragraph.\n").unwrap();
    let section = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Section { level, title, .. } => Some((*level, title.iter().map(Inline::plain_text).collect::<String>())),
            _ => None,
        })
        .expect("section");
    assert_eq!(section, (1, "Title".to_string()));
    let paragraphs = paragraph_text(&graph);
    assert_eq!(paragraphs, vec!["Intro text.".to_string(), "Second paragraph.".to_string()]);
}

#[test]
fn strong_and_emphasis_keep_plain_text_without_delimiters() {
    let graph = import_typst_bytes("styles.typ", "A *bold* and _soft_ word.\n").unwrap();
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
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "A bold and soft word.");
}

#[test]
fn math_keeps_authored_source() {
    let text = "Inline $x^2 + y^2$ math.\n\n$\n  x^2 + y^2\n$\n";
    let graph = import_typst_bytes("math.typ", text).unwrap();
    let inline_math: Vec<String> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
                Inline::InlineMath { formula } => {
                    assert_eq!(formula.language.as_deref(), Some("typst"));
                    Some(formula.source.clone())
                }
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(inline_math, vec!["x^2 + y^2".to_string()]);
    let display_math: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Math { formula } => {
                assert_eq!(formula.language.as_deref(), Some("typst"));
                Some(formula.source.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(display_math, vec!["x^2 + y^2"]);
}

/// The shared MathIR, with the language-relative spans removed.
///
/// Spans index each profile's own formula source, so the contract's
/// node-for-node equality compares the structure after one shared
/// normalization that clears them.
fn without_spans(expression: &MathExpression) -> MathExpression {
    use document_ir::{MathDelimiter, MathEnvironment, MathRow};
    match expression.clone() {
        MathExpression::Sequence(items) => MathExpression::Sequence(items.iter().map(without_spans).collect()),
        MathExpression::Atom(atom) => MathExpression::Atom(document_ir::MathAtom { span: None, ..atom }),
        MathExpression::Group { body } => MathExpression::Group { body: Box::new(without_spans(&body)) },
        MathExpression::Number { text, .. } => MathExpression::Number { text, span: None },
        MathExpression::Script { base, superscript, subscript } => MathExpression::Script {
            base: Box::new(without_spans(&base)),
            superscript: superscript.map(|value| Box::new(without_spans(&value))),
            subscript: subscript.map(|value| Box::new(without_spans(&value))),
        },
        MathExpression::Fraction { numerator, denominator, line } => MathExpression::Fraction {
            numerator: Box::new(without_spans(&numerator)),
            denominator: Box::new(without_spans(&denominator)),
            line,
        },
        MathExpression::Radical { index, radicand, .. } => MathExpression::Radical {
            index: index.map(|value| Box::new(without_spans(&value))),
            radicand: Box::new(without_spans(&radicand)),
            span: None,
        },
        MathExpression::Delimited { open, close, body, middles } => MathExpression::Delimited {
            open: open.map(|value| MathDelimiter { span: None, ..value }),
            close: close.map(|value| MathDelimiter { span: None, ..value }),
            body: Box::new(without_spans(&body)),
            middles: middles.iter().map(without_spans).collect(),
        },
        MathExpression::Styled { style, body } => MathExpression::Styled { style, body: Box::new(without_spans(&body)) },
        MathExpression::Accent { mark, wide, base, .. } => {
            MathExpression::Accent { mark, wide, base: Box::new(without_spans(&base)), span: None }
        }
        MathExpression::StackAbove { over, base } => {
            MathExpression::StackAbove { over: Box::new(without_spans(&over)), base: Box::new(without_spans(&base)) }
        }
        MathExpression::StackBelow { under, base } => {
            MathExpression::StackBelow { under: Box::new(without_spans(&under)), base: Box::new(without_spans(&base)) }
        }
        MathExpression::Relation(inner) => MathExpression::Relation(Box::new(without_spans(&inner))),
        MathExpression::Space { kind } => MathExpression::Space { kind },
        MathExpression::StyleSwitch { style } => MathExpression::StyleSwitch { style },
        MathExpression::LimitsPolicy { limits } => MathExpression::LimitsPolicy { limits },
        MathExpression::Environment(environment) => MathExpression::Environment(MathEnvironment {
            rows: environment.rows.iter().map(|row| MathRow { cells: row.cells.iter().map(without_spans).collect(), ..row.clone() }).collect(),
            span: None,
            ..environment
        }),
        MathExpression::Text { text, .. } => MathExpression::Text { text, span: None },
        MathExpression::Unsupported { kind, source, .. } => MathExpression::Unsupported { kind, source, span: None },
    }
}

/// Lower one Typst body through the real import and return its expression.
fn typst_expression(body: &str) -> MathExpression {
    let graph = import_typst_bytes("math.typ", &format!("${body}$")).expect("typst imports");
    let formula = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
                Inline::InlineMath { formula } => Some(formula.clone()),
                _ => None,
            }),
            Block::Math { formula } => Some(formula.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`{body}` must reach the IR as math"));
    assert_eq!(formula.language.as_deref(), Some("typst"), "the profile tag names the lowering");
    formula.expression
}

/// Contract 28 acceptance 1: the same construct lands on the same MathIR
/// node in both profiles after one span-clearing normalization.
#[test]
fn typst_math_lowers_to_the_shared_ir_nodes() {
    for (typst, tex) in [
        ("x^2 + 1", "x^2+1"),
        ("x_1", "x_1"),
        ("x^2_1", "x^2_1"),
        ("x_1^2", "x_1^2"),
        ("x^(2a)", "x^{2a}"),
        ("x^(2)", "x^{2}"),
        ("x^(a+b)", "x^{a+b}"),
        ("a = b", "a=b"),
        ("1/2", "1/2"),
        ("pi", r"\pi"),
        ("alpha + beta", r"\alpha + \beta"),
        ("sum", r"\sum"),
        ("int", r"\int"),
        ("le", r"\le"),
        ("f (x)", "f (x)"),
        ("x + y", "x+y"),
    ] {
        let from_typst = without_spans(&typst_expression(typst));
        let from_tex = without_spans(&document_formats::lower_tex_formula_source(tex, MathDisplay::Inline).expression);
        assert_eq!(from_typst, from_tex, "Typst `{typst}` and TeX `{tex}` must agree node for node");
    }
}

#[test]
fn lowered_math_keeps_the_authored_source_and_records_no_loss() {
    let graph = import_typst_bytes("math.typ", "$ x^2 + 1 $\n").unwrap();
    let formula = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Math { formula } => Some(formula),
            _ => None,
        })
        .expect("display math block");
    assert_eq!(formula.source, "x^2 + 1", "the authored body stays retrievable");
    assert_eq!(formula.display, MathDisplay::Display);
    assert!(graph.coverage.loss.is_empty(), "a successful lowering adds no diagnostics: {:?}", graph.coverage.loss);
}

/// Contract 28 acceptance 4: a construct outside the subset keeps its source
/// as an `Unsupported` node with a construct-level loss, and the import and
/// the surrounding content continue.
#[test]
fn out_of_subset_math_records_construct_losses_and_continues() {
    let text = "$ a [x] b $\n\nTail text.\n";
    let graph = import_typst_bytes("subset-gap.typ", text).unwrap();
    let formula = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Math { formula } => Some(formula),
            _ => None,
        })
        .expect("the math region still reaches the IR");
    let MathExpression::Sequence(items) = &formula.expression
    else {
        panic!("expected a sequence, got {:?}", formula.expression)
    };
    assert!(
        items.iter().any(|item| matches!(item, MathExpression::Unsupported { kind, .. } if kind == "content")),
        "the content block must stay visible as an unsupported node: {items:?}"
    );
    let index = loss_index(&graph, "import.typst.math.content");
    assert_eq!(graph.coverage.loss[index].status, document_ir::SemanticStatus::Partial);
    let span = graph.loss_span(index).expect("the construct loss carries its authored span");
    assert_eq!(&text[span.start as usize..span.end as usize], "[x]");
    assert!(paragraph_text(&graph).iter().any(|text| text == "Tail text."), "the page continues past the gap");
}

/// A `#` calculation fragment keeps its own construct loss while the rest of
/// the region lowers normally.
#[test]
fn calc_fragment_lowers_as_unsupported_with_a_construct_loss() {
    let text = "$a#b$\n";
    let graph = import_typst_bytes("calc.typ", text).unwrap();
    let formula = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
                Inline::InlineMath { formula } => Some(formula),
                _ => None,
            }),
            _ => None,
        })
        .expect("the math region reaches the IR");
    let MathExpression::Sequence(items) = &formula.expression
    else {
        panic!("expected a sequence, got {:?}", formula.expression)
    };
    assert!(matches!(&items[0], MathExpression::Atom(atom) if atom.text == "a"), "{items:?}");
    assert!(matches!(&items[1], MathExpression::Unsupported { kind, .. } if kind == "calc"), "{items:?}");
    let index = loss_index(&graph, "import.typst.math.calc");
    let span = graph.loss_span(index).expect("the calc loss carries its authored span");
    assert_eq!(&text[span.start as usize..span.end as usize], "#b");
}

#[test]
fn dotted_symbol_and_content_block_losses_keep_the_import_going() {
    let graph = import_typst_bytes("gaps.typ", "$ arrow.r $ and $ [x] $\n").unwrap();
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.typst.math.symbol"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.typst.math.content"));
    assert!(paragraph_text(&graph).iter().any(|text| text.contains("and")), "surrounding text survives");
}

/// Contract 28 acceptance 5: the inline / display decision follows the same
/// whitespace adjacency the TeX `$ ... $` profile uses.
#[test]
fn display_decision_follows_the_whitespace_adjacency() {
    let graph = import_typst_bytes("display.typ", "Text $x + y$ tail.\n\n$ a = b $\n").unwrap();
    let inline = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
                Inline::InlineMath { formula } => Some(formula),
                _ => None,
            }),
            _ => None,
        })
        .expect("unspaced math is inline");
    assert_eq!(inline.display, MathDisplay::Inline);
    assert!(matches!(inline.expression, MathExpression::Sequence(_)), "the inline body lowers: {:?}", inline.expression);
    let display = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Math { formula } => Some(formula),
            _ => None,
        })
        .expect("spaced math is a display block");
    assert_eq!(display.display, MathDisplay::Display);
    assert!(matches!(display.expression, MathExpression::Sequence(_)), "the display body lowers: {:?}", display.expression);
}

#[test]
fn fenced_raw_becomes_code_with_language() {
    let graph = import_typst_bytes("raw.typ", "```rust\nfn main() {}\n```\n").unwrap();
    let code = blocks(&graph)
        .iter()
        .find_map(|node| match &node.block {
            Block::Code { language, content } => Some((language.clone(), content.clone())),
            _ => None,
        })
        .expect("code block");
    assert_eq!(code, (Some("rust".to_string()), "fn main() {}".to_string()));
}

#[test]
fn unordered_and_ordered_lists_lower_with_numbers() {
    let graph = import_typst_bytes("lists.typ", "- One\n- Two\n\n1. First\n2. Second\n\nafter\n").unwrap();
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
    assert_eq!(
        items.iter().map(|item| item.content.iter().map(Inline::plain_text).collect::<String>()).collect::<Vec<_>>(),
        vec!["One".to_string(), "Two".to_string()]
    );
    let Block::List { ordered: true, items } = lists[1]
    else {
        panic!("expected ordered list, got {:?}", lists[1]);
    };
    assert_eq!(items.iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
    assert!(paragraph_text(&graph).iter().any(|text| text == "after"));
}

#[test]
fn hash_expression_stays_opaque_and_records_loss() {
    let graph = import_typst_bytes("expr.typ", "#figure(image(\"a.png\"))\n\nafter\n").unwrap();
    let opaque: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Opaque { kind, payload_hint, .. } => {
                assert!(payload_hint.contains("figure"));
                Some(kind.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(opaque, vec!["typst.expression.figure"]);
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.typst.dynamic_content"));
}

#[test]
fn empty_document_records_explicit_loss() {
    let graph = import_typst_bytes("empty.typ", "").unwrap();
    assert!(graph.blocks.is_empty());
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.typst.empty_document"));
}

/// The single opaque block of a document, together with its authored span.
fn only_opaque(graph: &DocumentGraph) -> (&str, &str, document_ir::SourceSpan) {
    let opaques: Vec<_> = blocks(graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Opaque { kind, payload_hint, .. } => Some((kind.as_str(), payload_hint.as_str(), node.id)),
            _ => None,
        })
        .collect();
    assert_eq!(opaques.len(), 1, "expected exactly one opaque block, got {opaques:?}");
    let (kind, payload, id) = opaques[0];
    let span = graph.block_span(id).expect("opaque block carries its authored span");
    (kind, payload, span)
}

fn loss_index(graph: &DocumentGraph, code: &str) -> usize {
    graph.coverage.loss.iter().position(|loss| loss.code == code).unwrap_or_else(|| panic!("missing loss `{code}`"))
}

#[test]
fn cross_line_figure_group_becomes_one_opaque_block() {
    let group = "#figure(\n  image(\"a.png\"),\n  caption: [A box],\n)";
    let text = format!("{group}\n\nafter\n");
    let graph = import_typst_bytes("figure.typ", &text).unwrap();

    let (kind, payload, span) = only_opaque(&graph);
    assert_eq!(kind, "typst.expression.figure");
    assert!(payload.contains("caption: [A box]"), "payload kept the whole group: {payload}");
    let covered = &text[span.start as usize..span.end as usize];
    assert_eq!(covered, group, "span must cover the construct from `#` through its closing bracket");

    assert_eq!(paragraph_text(&graph), vec!["after".to_string()], "construct body must never surface as body text");

    let index = loss_index(&graph, "import.typst.dynamic_content");
    assert_eq!(graph.coverage.loss[index].status, document_ir::SemanticStatus::Unsupported);
    assert_eq!(graph.loss_span(index), Some(span), "loss marker carries the same span as the opaque block");
}

#[test]
fn cross_line_rect_group_keeps_arguments_out_of_the_body() {
    let group = "#rect(\n  width: 2cm\n)";
    let text = format!("{group}\n\nafter\n");
    let graph = import_typst_bytes("rect.typ", &text).unwrap();

    let (kind, payload, span) = only_opaque(&graph);
    assert_eq!(kind, "typst.expression.rect");
    assert!(payload.contains("width: 2cm"), "payload kept the continuation line: {payload}");
    assert_eq!(&text[span.start as usize..span.end as usize], group);
    assert_eq!(paragraph_text(&graph), vec!["after".to_string()]);
}

#[test]
fn group_string_literal_parenthesis_does_not_close_the_group() {
    let group = "#text(\n  \"a ) b\",\n)";
    let text = format!("{group}\n\nafter\n");
    let graph = import_typst_bytes("string.typ", &text).unwrap();

    let (_, payload, span) = only_opaque(&graph);
    assert!(payload.contains("a ) b"));
    assert_eq!(&text[span.start as usize..span.end as usize], group, "the `)` inside the string literal must not end the group");
    assert_eq!(paragraph_text(&graph), vec!["after".to_string()]);
}

#[test]
fn group_comment_parenthesis_does_not_close_the_group() {
    let line_group = "#figure(\n  // ) not the closing bracket\n  image(\"a.png\"),\n)";
    let block_group = "#rect(\n  /* ) still inside the comment */\n  width: 2cm\n)";
    let text = format!("{line_group}\n\n{block_group}\n\nafter\n");
    let graph = import_typst_bytes("comment.typ", &text).unwrap();

    let opaques: Vec<&str> = blocks(&graph)
        .iter()
        .filter_map(|node| match &node.block {
            Block::Opaque { payload_hint, .. } => Some(payload_hint.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(opaques.len(), 2, "each construct stays one opaque block, got {opaques:?}");
    assert_eq!(&text[0..line_group.len()], line_group);
    assert_eq!(&text[line_group.len() + 2..line_group.len() + 2 + block_group.len()], block_group);
    assert_eq!(paragraph_text(&graph), vec!["after".to_string()]);
}

#[test]
fn unterminated_group_is_diagnosed_instead_of_swallowed() {
    let text = "#figure(\n  image(\"a.png\"),\n";
    let graph = import_typst_bytes("unterminated.typ", text).unwrap();

    let (_, payload, span) = only_opaque(&graph);
    assert!(payload.contains("image(\"a.png\")"));
    assert_eq!(&text[span.start as usize..span.end as usize], text, "span covers the unclosed group to the end of the region");

    let index = loss_index(&graph, "import.typst.unterminated_dynamic_group");
    assert_eq!(graph.coverage.loss[index].status, document_ir::SemanticStatus::Unsupported);
    assert_eq!(graph.loss_span(index), Some(span));
    assert!(paragraph_text(&graph).is_empty(), "no construct fragment may leak into the body");
}

#[test]
fn block_in_inline_context_loss_carries_the_authored_span() {
    let text = "Lead *a\n- b*\n";
    let graph = import_typst_bytes("inline-block.typ", text).unwrap();

    let index = loss_index(&graph, "import.typst.block_in_inline_context");
    let span = graph.loss_span(index).expect("block-in-inline loss must carry its authored span");
    assert_eq!(&text[span.start as usize..span.end as usize], "- b", "the span must name the rejected block construct");
}

#[test]
fn repeated_inline_blocks_keep_their_own_spans() {
    let text = "*a\n- b\nx\n- b*\n";
    let graph = import_typst_bytes("twice.typ", text).unwrap();

    let spans: Vec<document_ir::SourceSpan> = (0..graph.coverage.loss.len())
        .filter(|index| graph.coverage.loss[*index].code == "import.typst.block_in_inline_context")
        .map(|index| graph.loss_span(index).expect("each repeated payload keeps its own span"))
        .collect();
    assert_eq!(spans.len(), 2, "{:?}", graph.coverage.loss);
    for span in &spans {
        assert_eq!(&text[span.start as usize..span.end as usize], "- b");
    }
    assert_ne!(spans[0], spans[1], "the two occurrences must not merge onto the first one");
}

#[test]
fn cross_line_group_after_a_spaced_head_is_one_opaque_block() {
    let group = "#set page(\n  width: 10cm,\n)";
    let text = format!("= Intro\n\n{group}\n\nBody.\n");
    let graph = import_typst_bytes("spaced-head.typ", &text).unwrap();

    let (kind, payload, span) = only_opaque(&graph);
    assert_eq!(kind, "typst.expression.set");
    assert_eq!(payload, group, "the payload covers the whole group including its line breaks");
    assert_eq!(&text[span.start as usize..span.end as usize], group, "the span slices exactly the authored group");

    let dynamic_losses = graph.coverage.loss.iter().filter(|marker| marker.code == "import.typst.dynamic_content").count();
    assert_eq!(dynamic_losses, 1, "one group is one loss: {:?}", graph.coverage.loss);
    let index = loss_index(&graph, "import.typst.dynamic_content");
    assert_eq!(graph.loss_span(index), Some(span), "the loss span covers the complete group");

    assert!(blocks(&graph).iter().any(|node| matches!(&node.block, Block::Section { .. })), "the heading before the group survives");
    assert_eq!(paragraph_text(&graph), vec!["Body.".to_string()], "the group body never surfaces as body text");
}
