use panduck_core::AST;
use panduck_core::parse_markdown;

#[test]
fn empty() {
    let ast = parse_markdown("").unwrap();
    assert_eq!(ast, AST::statements(vec![]))
}

#[test]
fn test() {
    let ast = parse_markdown("text");
    println!("{:#?}", ast.unwrap_or_default())
}
