use panduck_rst::{tokenize, RstReadConfig, parse_and_generate_rst};
use panduck_types::AdapterError;

#[test]
fn rst_tokenize_is_not_implemented() {
    let err = tokenize("Title\n=====").unwrap_err();
    assert!(matches!(
        err,
        AdapterError::NotImplemented { feature } if feature == "rst lexer"
    ));
}

#[test]
fn rst_pipeline_is_not_implemented() {
    let err = parse_and_generate_rst("Title\n=====", RstReadConfig { support_math: false })
        .unwrap_err();
    assert!(matches!(err, AdapterError::NotImplemented { .. }));
}
