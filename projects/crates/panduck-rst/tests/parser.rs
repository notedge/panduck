use panduck_rst::{parse, ParserState};
use panduck_types::helpers::SourceText;
use panduck_types::AdapterError;

#[test]
fn rst_parser_is_not_implemented() {
    let source = SourceText::new("paragraph", None);
    let err = parse(vec![], source).unwrap_err();
    assert!(matches!(
        err,
        AdapterError::NotImplemented { feature } if feature == "rst parser"
    ));
    let _ = ParserState;
}
