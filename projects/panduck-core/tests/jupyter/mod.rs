use notedown_ast::ASTKind;
use panduck_core::convert::parse_jupyter;

#[test]
fn empty() {
    let ast = parse_jupyter("{}").unwrap();
    assert_eq!(ast, ASTKind::statements(vec![], None))
}
