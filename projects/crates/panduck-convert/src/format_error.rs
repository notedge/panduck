use notedown_formats::FormatError;
use panduck_types::AdapterError;

pub(crate) fn map_format_error(error: FormatError) -> AdapterError {
    match error {
        FormatError::NotImplemented { format, direction } => {
            AdapterError::not_implemented(format!("{direction} for {format}"))
        }
        FormatError::InvalidInput { message } => AdapterError::invalid_input(message),
        FormatError::Parse { format, message } => AdapterError::adapter(format, message),
        FormatError::Unsupported { format, operation } => {
            AdapterError::unsupported_format(format, operation)
        }
    }
}
