use panduck_core::{parse_jupyter, AST};

#[test]
fn empty() {
    let ast = parse_jupyter("{}").unwrap();
    assert_eq!(ast, AST::statements(vec![]))
}

#[test]
fn md_full() {
    let ast = parse_jupyter(include_str!("ipython.json"));
    println!("{:#?}", ast.unwrap_or_default())
}
