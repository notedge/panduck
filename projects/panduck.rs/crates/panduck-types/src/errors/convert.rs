use super::AdapterError;

impl From<std::io::Error> for AdapterError {
    fn from(source: std::io::Error) -> Self {
        AdapterError::io(source, None::<String>)
    }
}

impl From<std::fmt::Error> for AdapterError {
    fn from(_: std::fmt::Error) -> Self {
        AdapterError::adapter("writer", "formatting failed")
    }
}
