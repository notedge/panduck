use panduck_org::ast::OrgRoot;
use panduck_org::writer::OrgWriteConfig;
use panduck_types::AdapterError;

#[test]
fn org_writer_is_not_implemented() {
    let err = OrgWriteConfig::default()
        .writer(String::new())
        .generate(&OrgRoot { blocks: vec![] })
        .unwrap_err();
    assert!(matches!(
        err,
        AdapterError::NotImplemented { feature } if feature == "org writer"
    ));
}
