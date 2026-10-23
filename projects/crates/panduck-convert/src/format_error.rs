use notedown_formats::{FormatError, from_format_error};
use panduck_types::{AdapterError, Result};

pub(crate) fn fail<T>(error: AdapterError) -> Result<T> {
    panduck_diagnostic::log_adapter_error(&error);
    Err(error)
}

pub(crate) fn propagate<T>(result: Result<T>) -> Result<T> {
    if let Err(error) = &result {
        panduck_diagnostic::log_adapter_error(error);
    }
    result
}

/// Convert a Notedown [`FormatError`] into an [`AdapterError`] while preserving
/// the structured diagnostic code, typed args, and origin from the formats
/// layer. This avoids the previous pattern of stringifying the error into a
/// free-form message and losing the stable wire identifiers.
pub(crate) fn map_format_error(error: FormatError) -> AdapterError {
    let diagnostic = from_format_error(&error);
    AdapterError::diagnostic(diagnostic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boost_diagnostic::DiagnosticCode;

    fn extract_inner_diag(error: &AdapterError) -> &boost_diagnostic::Diagnostic {
        match error {
            AdapterError::Diagnostic { diagnostic } => diagnostic,
            _ => panic!("expected Diagnostic variant"),
        }
    }

    #[test]
    fn parse_error_preserves_notedown_code_and_origin() {
        let adapter = map_format_error(FormatError::parse("docx", "numbering.xml missing"));
        let diag = extract_inner_diag(&adapter);
        assert_eq!(diag.code(), &DiagnosticCode::new("notedown.format.parse-failed"));
        // origin must stay at notedown/formats — not remapped to panduck/adapter
        assert_eq!(diag.origin().namespace(), "notedown");
        assert_eq!(diag.origin().component(), "formats");
        // args must include both format identifier and failure reason
        let args: Vec<_> = diag.message().args().iter().map(|(k, _)| k.clone()).collect();
        assert!(args.contains(&"format".to_string()));
        assert!(args.contains(&"message".to_string()));
    }

    #[test]
    fn unsupported_error_preserves_format_and_operation_args() {
        let adapter = map_format_error(FormatError::unsupported("pdf", "hyperlinks"));
        let diag = extract_inner_diag(&adapter);
        assert_eq!(diag.code(), &DiagnosticCode::new("notedown.format.unsupported"));
        let args: Vec<_> = diag.message().args().iter().map(|(k, _)| k.clone()).collect();
        assert!(args.contains(&"format".to_string()));
        assert!(args.contains(&"operation".to_string()));
    }

    #[test]
    fn invalid_input_error_preserves_message_arg() {
        let adapter = map_format_error(FormatError::invalid_input("zero-length bytes"));
        let diag = extract_inner_diag(&adapter);
        assert_eq!(diag.code(), &DiagnosticCode::new("notedown.format.invalid-input"));
    }

    #[test]
    fn budget_error_preserves_format_and_message_args() {
        let adapter = map_format_error(FormatError::budget("epub", "zip member exceeds 2 MiB"));
        let diag = extract_inner_diag(&adapter);
        assert_eq!(diag.code(), &DiagnosticCode::new("notedown.format.budget-exhausted"));
        let args: Vec<_> = diag.message().args().iter().map(|(k, _)| k.clone()).collect();
        assert!(args.contains(&"format".to_string()));
        assert!(args.contains(&"message".to_string()));
    }

    #[test]
    fn not_implemented_error_preserves_format_and_direction_args() {
        let adapter = map_format_error(FormatError::not_implemented("pdf", "export"));
        let diag = extract_inner_diag(&adapter);
        assert_eq!(diag.code(), &DiagnosticCode::new("notedown.format.not-implemented"));
        let args: Vec<_> = diag.message().args().iter().map(|(k, _)| k.clone()).collect();
        assert!(args.contains(&"format".to_string()));
        assert!(args.contains(&"direction".to_string()));
    }

    #[test]
    fn from_adapter_error_passthrough_preserves_code_and_origin() {
        let adapter = map_format_error(FormatError::parse("docx", "numbering.xml missing"));
        // panduck-diagnostic::from_adapter_error must not remap the Diagnostic
        let reprojected = panduck_diagnostic::from_adapter_error(&adapter);
        assert_eq!(reprojected.code(), &DiagnosticCode::new("notedown.format.parse-failed"));
        assert_eq!(reprojected.origin().namespace(), "notedown");
        assert_eq!(reprojected.origin().component(), "formats");
    }
}
