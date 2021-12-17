use notedown_ast::ASTKind;
use panduck_core::convert::parse_common_markdown;

#[test]
fn empty() {
    let ast = parse_common_markdown("").unwrap();
    assert_eq!(ast, ASTKind::statements(vec![], None))
}
