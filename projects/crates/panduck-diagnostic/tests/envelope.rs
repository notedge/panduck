use panduck_diagnostic::{envelope_from_adapter_error, from_adapter_error};
use panduck_types::AdapterError;

#[test]
fn envelope_from_adapter_error_contains_one_diagnostic() {
    let error = AdapterError::unsupported_format("doc", "read");
    let envelope = envelope_from_adapter_error(&error);
    let diagnostic = from_adapter_error(&error);
    assert_eq!(envelope.diagnostics.len(), 1);
    assert_eq!(envelope.diagnostics[0].code().as_str(), diagnostic.code().as_str());
}
