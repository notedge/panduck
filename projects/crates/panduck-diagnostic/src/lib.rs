#![warn(missing_docs)]
//! Panduck orchestration diagnostics on the shared `diagnostic` contract.

use diagnostic::{
    Diagnostic, DiagnosticCode as WireDiagnosticCode, DiagnosticOrigin, DiagnosticSeverity,
    DiagnosticSet, Message,
};
use notedown_ir::{DocumentGraph, LossMarker, SemanticStatus};
use panduck_types::AdapterError;

/// Panduck domain diagnostic code (maps to dotted `panduck.*` wire identifiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    AdapterIo,
    AdapterInvalidInput,
    AdapterInvalidRange,
    AdapterNotImplemented,
    AdapterFailure,
    AdapterUnsupportedFormat,
    AdapterConfig,
    SemanticLoss,
}

impl DiagnosticCode {
    /// Stable dotted wire identifier for this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdapterIo => "panduck.adapter.io",
            Self::AdapterInvalidInput => "panduck.adapter.invalid-input",
            Self::AdapterInvalidRange => "panduck.adapter.invalid-range",
            Self::AdapterNotImplemented => "panduck.adapter.not-implemented",
            Self::AdapterFailure => "panduck.adapter.failure",
            Self::AdapterUnsupportedFormat => "panduck.adapter.unsupported-format",
            Self::AdapterConfig => "panduck.adapter.config",
            Self::SemanticLoss => "panduck.semantic.loss",
        }
    }

    /// Producer component for [`DiagnosticOrigin`].
    pub const fn component(self) -> &'static str {
        match self {
            Self::SemanticLoss => "semantic",
            _ => "adapter",
        }
    }

    /// Convert to the shared wire diagnostic code type.
    pub fn wire_code(self) -> WireDiagnosticCode {
        WireDiagnosticCode::new(self.as_str())
    }
}

pub use diagnostic::{
    Diagnostic as UnifiedDiagnostic, DiagnosticAction, DiagnosticEnvelope, DiagnosticLabel,
    DiagnosticLocation, DiagnosticSet as UnifiedDiagnosticSet, DiagnosticSink, MessageArg,
    SCHEMA_VERSION as DIAGNOSTIC_SCHEMA_VERSION,
};

/// Build a Panduck error diagnostic with fallback text.
pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Diagnostic {
    build(code, DiagnosticSeverity::Error, message)
}

/// Build a Panduck warning diagnostic with fallback text.
pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Diagnostic {
    build(code, DiagnosticSeverity::Warning, message)
}

fn build(code: DiagnosticCode, severity: DiagnosticSeverity, message: impl Into<String>) -> Diagnostic {
    let fallback = message.into();
    Diagnostic::new(
        code.wire_code(),
        severity,
        DiagnosticOrigin::new("panduck", code.component()),
        Message::new(code.as_str()).with_fallback(fallback),
    )
}

fn loss_severity(status: SemanticStatus) -> DiagnosticSeverity {
    match status {
        SemanticStatus::Resolved => DiagnosticSeverity::Info,
        SemanticStatus::Inferred | SemanticStatus::Partial => DiagnosticSeverity::Warning,
        SemanticStatus::Lossy | SemanticStatus::Unsupported => DiagnosticSeverity::Warning,
        SemanticStatus::Unresolved => DiagnosticSeverity::Error,
    }
}

/// Map one IR loss marker to a unified diagnostic.
pub fn from_loss_marker(marker: &LossMarker) -> Diagnostic {
    let code = if marker.code.contains('.') {
        WireDiagnosticCode::new(&marker.code)
    } else {
        DiagnosticCode::SemanticLoss.wire_code()
    };
    Diagnostic::new(
        code,
        loss_severity(marker.status),
        DiagnosticOrigin::new("panduck", "semantic"),
        Message::new("panduck.semantic.loss").with_fallback(marker.message.clone()),
    )
}

/// Map adapter-layer failures to unified diagnostics.
pub fn from_adapter_error(error: &AdapterError) -> Diagnostic {
    let (code, fallback) = match error {
        AdapterError::Io { source, path } => {
            let detail = match path {
                Some(path) => format!("I/O error at {path}: {source}"),
                None => format!("I/O error: {source}"),
            };
            (DiagnosticCode::AdapterIo, detail)
        }
        AdapterError::InvalidInput { message } => {
            (DiagnosticCode::AdapterInvalidInput, message.clone())
        }
        AdapterError::InvalidRange { offset, len } => {
            (
                DiagnosticCode::AdapterInvalidRange,
                format!("invalid source range at {offset} len {len}"),
            )
        }
        AdapterError::NotImplemented { feature } => {
            (
                DiagnosticCode::AdapterNotImplemented,
                format!("not implemented: {feature}"),
            )
        }
        AdapterError::Adapter { adapter, message } => {
            (
                DiagnosticCode::AdapterFailure,
                format!("{adapter}: {message}"),
            )
        }
        AdapterError::UnsupportedFormat { format, operation } => {
            (
                DiagnosticCode::AdapterUnsupportedFormat,
                format!("{format} does not support {operation}"),
            )
        }
        AdapterError::Config { path, message } => {
            let detail = match path {
                Some(path) => format!("config error at {path}: {message}"),
                None => format!("config error: {message}"),
            };
            (DiagnosticCode::AdapterConfig, detail)
        }
    };
    build(code, DiagnosticSeverity::Error, fallback)
}

/// Collect semantic loss diagnostics from a document graph.
pub fn diagnostics_from_graph(graph: &DocumentGraph) -> DiagnosticSet {
    let mut set = DiagnosticSet::new();
    for marker in &graph.coverage.loss {
        set.push(from_loss_marker(marker));
    }
    set
}

#[cfg(feature = "console")]
pub use diagnostic::diagnostic_to_event;

/// Project a Panduck diagnostic into a console event.
#[cfg(feature = "console")]
pub fn to_console_event(diagnostic: &Diagnostic) -> console::ConsoleEvent {
    diagnostic_to_event(diagnostic)
}

/// Project an adapter failure into a console event.
#[cfg(feature = "console")]
pub fn adapter_error_to_event(error: &AdapterError) -> console::ConsoleEvent {
    diagnostic_to_event(&from_adapter_error(error))
}

/// Emit one unified diagnostic through the global console facade.
#[cfg(feature = "console")]
pub fn emit_diagnostic(diagnostic: &Diagnostic) {
    console::emit(to_console_event(diagnostic));
}

/// Emit an adapter failure as a structured diagnostic console event.
#[cfg(feature = "console")]
pub fn emit_adapter_error(error: &AdapterError) {
    console::emit(adapter_error_to_event(error));
}

/// Emit every diagnostic in a set through the global console facade.
#[cfg(feature = "console")]
pub fn emit_diagnostic_set(set: &DiagnosticSet) {
    for diagnostic in set.diagnostics() {
        emit_diagnostic(diagnostic);
    }
}
