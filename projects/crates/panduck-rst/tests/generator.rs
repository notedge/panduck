use panduck_rst::{generate, RstRoot};
use panduck_types::AdapterError;

#[test]
fn rst_writer_is_not_implemented() {
    let err = generate(RstRoot { blocks: vec![] }).unwrap_err();
    assert!(matches!(
        err,
        AdapterError::NotImplemented { feature } if feature == "rst writer"
    ));
}
