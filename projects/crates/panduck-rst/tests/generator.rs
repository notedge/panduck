use panduck_rst::lexer::tokenize;
use panduck_rst::parser::parse;
use panduck_rst::generator::generate;
use panduck_types::helpers::SourceText;

#[test]
fn test_heading_generator() {
    let source = SourceText::new("Title\n=======");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "Title\n=======\n");
}

#[test]
fn test_paragraph_generator() {
    let source = SourceText::new("This is a paragraph.");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "This is a paragraph.\n\n");
}

#[test]
fn test_bold_generator() {
    let source = SourceText::new("**bold text**");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "**bold text**\n\n");
}

#[test]
fn test_italic_generator() {
    let source = SourceText::new("*italic text*");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "*italic text*\n\n");
}

#[test]
fn test_code_generator() {
    let source = SourceText::new("``code``");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "``code``\n\n");
}

#[test]
fn test_link_generator() {
    let source = SourceText::new("`link text <http://example.com>`_`");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "`link text <http://example.com>`_\n\n");
}

#[test]
fn test_unordered_list_generator() {
    let source = SourceText::new("- Item 1\n- Item 2");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "- Item 1\n- Item 2\n\n");
}

#[test]
fn test_ordered_list_generator() {
    let source = SourceText::new("1. Item 1\n2. Item 2");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, _parser_diagnostics) = parse(tokens, source);
    let (rst_output, _generator_diagnostics) = generate(ast);

    assert_eq!(rst_output, "1. Item 1\n2. Item 2\n\n");
}