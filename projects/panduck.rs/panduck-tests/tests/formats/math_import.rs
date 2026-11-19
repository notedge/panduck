//! Structured math survives DOCX OMML and HTML/EPUB MathML import.
//!
//! Both authoring formats are lowered to the same `MathFormula` representation
//! as authored TeX. Unknown constructs must keep their readable text and leave
//! a loss marker instead of dropping the whole formula.

#![cfg(any(feature = "docx", feature = "html", feature = "epub"))]

#[cfg(feature = "docx")]
use std::collections::HashMap;

use document_ir::{Block, DocumentGraph, Inline, MathDisplay, MathExpression, SemanticStatus};

fn block_math(formula_block: &Block) -> Option<&document_ir::MathFormula> {
    match formula_block {
        Block::Math { formula } => Some(formula),
        _ => None,
    }
}

fn count_block_math(graph: &DocumentGraph) -> usize {
    graph.blocks.iter().filter(|node| block_math(&node.block).is_some()).count()
}

fn all_inline_math(graph: &DocumentGraph) -> Vec<&document_ir::MathFormula> {
    let mut formulas = Vec::new();
    for node in &graph.blocks {
        match &node.block {
            Block::Paragraph { content } => {
                formulas.extend(content.iter().filter_map(|inline| match inline {
                    Inline::InlineMath { formula } => Some(formula),
                    _ => None,
                }));
            }
            Block::Math { formula } => formulas.push(formula),
            _ => {}
        }
    }
    formulas
}

fn has_loss(graph: &DocumentGraph, code: &str) -> bool {
    graph.coverage.loss.iter().any(|marker| marker.code == code)
}

fn contains_expression(expression: &MathExpression, predicate: &impl Fn(&MathExpression) -> bool) -> bool {
    if predicate(expression) {
        return true;
    }
    match expression {
        MathExpression::Sequence(items) => items.iter().any(|item| contains_expression(item, predicate)),
        MathExpression::Script { base, superscript, subscript } => {
            contains_expression(base, predicate)
                || superscript.as_deref().is_some_and(|item| contains_expression(item, predicate))
                || subscript.as_deref().is_some_and(|item| contains_expression(item, predicate))
        }
        MathExpression::Fraction { numerator, denominator, .. } => {
            contains_expression(numerator, predicate) || contains_expression(denominator, predicate)
        }
        MathExpression::Radical { index, radicand, .. } => {
            index.as_deref().is_some_and(|item| contains_expression(item, predicate)) || contains_expression(radicand, predicate)
        }
        MathExpression::Delimited { body, middles, .. } => {
            contains_expression(body, predicate) || middles.iter().any(|item| contains_expression(item, predicate))
        }
        MathExpression::Styled { body, .. } | MathExpression::Group { body } | MathExpression::Relation(body) => {
            contains_expression(body, predicate)
        }
        MathExpression::Accent { base, .. } => contains_expression(base, predicate),
        MathExpression::StackAbove { over, base } => contains_expression(over, predicate) || contains_expression(base, predicate),
        MathExpression::StackBelow { under, base } => contains_expression(under, predicate) || contains_expression(base, predicate),
        MathExpression::Environment(environment) => {
            environment.rows.iter().flat_map(|row| row.cells.iter()).any(|cell| contains_expression(cell, predicate))
        }
        _ => false,
    }
}

#[cfg(feature = "docx")]
mod docx_omml {
    use super::*;
    use document_formats::import::docx::{numbering::NumberingCatalog, styles::StyleDefaults, xml};

    fn import(source: &str) -> DocumentGraph {
        let mut graph = xml::new_graph("math.docx");
        xml::parse_document_xml(
            source.as_bytes(),
            &HashMap::new(),
            &NumberingCatalog::default(),
            &HashMap::new(),
            &StyleDefaults::default(),
            &mut graph,
        )
        .expect("docx body parse");
        graph
    }

    fn document(body: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"><w:body>{body}</w:body></w:document>"#
        )
    }

    #[test]
    fn supported_omml_constructs_become_one_structured_math_formula() {
        let body = r#"<w:p><m:oMathPara><m:oMath><m:sSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSup></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:sSubSup><m:e><m:r><m:t>x</m:t></m:r></m:e><m:sub><m:r><m:t>1</m:t></m:r></m:sub><m:sup><m:r><m:t>2</m:t></m:r></m:sup></m:sSubSup></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:f><m:num><m:r><m:t>a</m:t></m:r></m:num><m:den><m:r><m:t>b</m:t></m:r></m:den></m:f></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:rad><m:deg><m:r><m:t>3</m:t></m:r></m:deg><m:e><m:r><m:t>x</m:t></m:r></m:e></m:rad></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:d><m:dPr><m:begChr m:val="["/><m:endChr m:val="]"/></m:dPr><m:e><m:r><m:t>y</m:t></m:r></m:e></m:d></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr><m:sub><m:r><m:t>i=0</m:t></m:r></m:sub><m:sup><m:r><m:t>n</m:t></m:r></m:sup><m:e><m:r><m:t>x</m:t></m:r></m:e></m:nary></m:oMath></m:oMathPara></w:p>
<w:p><m:oMathPara><m:oMath><m:m><m:mr><m:e><m:r><m:t>1</m:t></m:r></m:e><m:e><m:r><m:t>2</m:t></m:r></m:e></m:mr><m:mr><m:e><m:r><m:t>3</m:t></m:r></m:e><m:e><m:r><m:t>4</m:t></m:r></m:e></m:mr></m:m></m:oMath></m:oMathPara></w:p>"#;
        let graph = import(&document(body));
        assert_eq!(count_block_math(&graph), 7, "{:?}", graph.blocks.iter().map(|node| &node.block).collect::<Vec<_>>());
        let formulas: Vec<&document_ir::MathFormula> = graph.blocks.iter().filter_map(|node| block_math(&node.block)).collect();
        for formula in &formulas {
            assert_eq!(formula.display, MathDisplay::Display);
        }
        let sources: Vec<&str> = formulas.iter().map(|formula| formula.source.as_str()).collect();
        assert!(sources.iter().any(|source| source.contains("^{2}")), "{sources:?}");
        assert!(sources.iter().any(|source| source.contains("\\frac{")), "{sources:?}");
        assert!(sources.iter().any(|source| source.contains("\\sqrt[3]{")), "{sources:?}");
        assert!(sources.iter().any(|source| source.contains("\\left[")), "{sources:?}");
        assert!(sources.iter().any(|source| source.contains("\\sum_{")), "{sources:?}");
        assert!(sources.iter().any(|source| source.contains("\\begin{matrix}")), "{sources:?}");
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Script { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Fraction { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Radical { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Delimited { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Environment(_))))
        );
        assert!(!has_loss(&graph, "import.docx.math_unsupported"), "{:?}", graph.coverage.loss);
    }

    #[test]
    fn unknown_omml_construct_keeps_text_and_reports_a_loss() {
        let body = r#"<w:p><m:oMathPara><m:oMath><m:customConstruct><m:r><m:t>kept text</m:t></m:r></m:customConstruct></m:oMath></m:oMathPara></w:p>"#;
        let graph = import(&document(body));
        assert_eq!(count_block_math(&graph), 1, "{:?}", graph.blocks.iter().map(|node| &node.block).collect::<Vec<_>>());
        let formula = graph.blocks.iter().find_map(|node| block_math(&node.block)).expect("formula survived");
        assert!(formula.source.contains("kept text"), "{}", formula.source);
        let marker =
            graph.coverage.loss.iter().find(|marker| marker.code == "import.docx.math_unsupported").expect("unsupported OMML must be reported");
        assert_eq!(marker.status, SemanticStatus::Partial);
        assert!(marker.message.contains("omml.customConstruct"), "{}", marker.message);
    }
}

#[cfg(feature = "html")]
mod html_mathml {
    use super::*;
    use document_formats::import::html::import_html_text;

    fn import(source: &str) -> DocumentGraph {
        import_html_text("math.html", source).expect("html import")
    }

    #[test]
    fn supported_mathml_constructs_become_structured_formulas() {
        let source = r#"<p>inline <math><msup><mi>x</mi><mn>2</mn></msup></math></p>
<p><math display="block"><mfrac><mi>a</mi><mi>b</mi></mfrac></math></p>
<p><math><msqrt><mi>x</mi></msqrt><mfenced open="[" close="]"><mi>y</mi></mfenced></math></p>
<p><math><msubsup><mo>∫</mo><mn>0</mn><mn>1</mn></msubsup></math></p>
<p><math><mtable><mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr><mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr></mtable></math></p>"#;
        let graph = import(source);
        assert_eq!(count_block_math(&graph), 1, "{:?}", graph.blocks.iter().map(|node| &node.block).collect::<Vec<_>>());
        let formulas = all_inline_math(&graph);
        assert_eq!(formulas.len(), 5, "{:?}", formulas.iter().map(|formula| &formula.source).collect::<Vec<_>>());
        assert_eq!(formulas[0].display, MathDisplay::Inline);
        assert_eq!(formulas[1].display, MathDisplay::Display);
        assert!(formulas.iter().any(|formula| formula.source.contains("^{2}")), "{:?}", formulas.iter().map(|f| &f.source).collect::<Vec<_>>());
        assert!(
            formulas.iter().any(|formula| formula.source.contains("\\frac{")),
            "{:?}",
            formulas.iter().map(|f| &f.source).collect::<Vec<_>>()
        );
        assert!(
            formulas.iter().any(|formula| formula.source.contains("\\sqrt{")),
            "{:?}",
            formulas.iter().map(|f| &f.source).collect::<Vec<_>>()
        );
        assert!(
            formulas.iter().any(|formula| formula.source.contains("\\left[")),
            "{:?}",
            formulas.iter().map(|f| &f.source).collect::<Vec<_>>()
        );
        assert!(
            formulas.iter().any(|formula| formula.source.contains("\\int}") && formula.source.contains("_{0}^{1}")),
            "{:?}",
            formulas.iter().map(|f| &f.source).collect::<Vec<_>>()
        );
        assert!(
            formulas.iter().any(|formula| formula.source.contains("\\begin{matrix}")),
            "{:?}",
            formulas.iter().map(|f| &f.source).collect::<Vec<_>>()
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Fraction { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Radical { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Delimited { .. })))
        );
        assert!(
            formulas
                .iter()
                .any(|formula| contains_expression(&formula.expression, &|expression| matches!(expression, MathExpression::Environment(_))))
        );
        assert!(!has_loss(&graph, "import.html.math_unsupported"), "{:?}", graph.coverage.loss);
    }

    #[test]
    fn compound_mathml_formula_keeps_the_integral_and_fraction() {
        let source = r#"<p><math display="block"><mrow><mi>E</mi><mo>=</mo><mi>m</mi><msup><mi>c</mi><mn>2</mn></msup><mo>,</mo><mspace width="1em"/><msubsup><mo>∫</mo><mn>0</mn><mn>1</mn></msubsup><msup><mi>x</mi><mn>2</mn></msup><mi>dx</mi><mo>=</mo><mfrac><mn>1</mn><mn>3</mn></mfrac></mrow></math></p><p>tail survives</p>"#;
        let graph = import(source);
        let formulas = all_inline_math(&graph);
        assert_eq!(formulas.len(), 1, "{:?}", formulas.iter().map(|formula| &formula.source).collect::<Vec<_>>());
        let formula_source = &formulas[0].source;
        assert!(formula_source.contains("\\quad"), "{formula_source}");
        assert!(formula_source.contains("\\int}") && formula_source.contains("_{0}^{1}"), "{formula_source}");
        assert!(formula_source.contains("\\frac{1}{3}"), "{formula_source}");
        assert!(formula_source.contains("dx"), "{formula_source}");
        assert!(
            contains_expression(&formulas[0].expression, &|expression| matches!(expression, MathExpression::Fraction { .. })),
            "{formula_source}"
        );
        let tail = graph.blocks.iter().any(|node| match &node.block {
            Block::Paragraph { content } => {
                content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("tail survives")))
            }
            _ => false,
        });
        assert!(tail, "content after the formula must stay in the document");
        assert!(!has_loss(&graph, "import.html.math_unsupported"), "{:?}", graph.coverage.loss);
    }

    #[test]
    fn unknown_mathml_construct_keeps_text_and_reports_a_loss() {
        let graph = import(r#"<p><math><mo>+</mo><ms>kept text</ms></math></p>"#);
        let formulas = all_inline_math(&graph);
        assert_eq!(formulas.len(), 1, "{:?}", graph.blocks.iter().map(|node| &node.block).collect::<Vec<_>>());
        assert!(formulas[0].source.contains("kept text"), "{}", formulas[0].source);
        let marker = graph
            .coverage
            .loss
            .iter()
            .find(|marker| marker.code == "import.html.math_unsupported")
            .expect("unsupported MathML must be reported");
        assert_eq!(marker.status, SemanticStatus::Partial);
        assert!(marker.message.contains("mathml.ms"), "{}", marker.message);
    }
}
