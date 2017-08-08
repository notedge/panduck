use panduck_org::reader::OrgReadConfig;
use panduck_types::AdapterError;

#[test]
fn org_parser_is_not_implemented() {
    let err = OrgReadConfig {
        support_math: false,
    }
    .read_str("* title", None)
    .unwrap_err();
    assert!(matches!(err, AdapterError::NotImplemented { .. }));
}
