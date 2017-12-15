#![deny(missing_docs)]
//! Panduck orchestration diagnostics on the shared `boost-diagnostic` contract.

use boost_diagnostic::{
    Diagnostic, DiagnosticCode as WireDiagnosticCode, DiagnosticOrigin, DiagnosticSet, DiagnosticSeverity, Message,
};
use notedown_ir::{DocumentGraph, LossMarker, SemanticStatus};
use panduck_types::AdapterError;

/// Panduck report schema (pre-1.0 — bump only on breaking wire changes).
pub const SCHEMA_REPORT: &str = "panduck.report/v0";

/// Panduck domain diagnostic code (maps to dotted `panduck.*` wire identifiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    /// Adapter or filesystem I/O failure.
    AdapterIo,
    /// Caller supplied invalid bytes, paths, or parameters.
    AdapterInvalidInput,
    /// Source text slice is out of range.
    AdapterInvalidRange,
    /// Adapter capability is not implemented in this build.
    AdapterNotImplemented,
    /// Named format adapter reported a failure.
    AdapterFailure,
    /// Target format cannot perform the requested operation.
    AdapterUnsupportedFormat,
    /// Configuration could not be loaded or validated.
    AdapterConfig,
    /// Semantic information was lost during conversion.
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

pub use boost_diagnostic::{
    Diagnostic as UnifiedDiagnostic, DiagnosticAction, DiagnosticEnvelope, DiagnosticLabel, DiagnosticLocation,
    DiagnosticSet as UnifiedDiagnosticSet, DiagnosticSink, MessageArg, SCHEMA_VERSION as DIAGNOSTIC_SCHEMA_VERSION,
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
    }
    else {
        DiagnosticCode::SemanticLoss.wire_code()
    };
    let msg = Message::new("panduck.semantic.loss")
        .with_arg("code", MessageArg::Text(marker.code.clone()))
        .with_arg("severity", MessageArg::Text(format!("{:?}", marker.status)));
    Diagnostic::new(
        code,
        loss_severity(marker.status),
        DiagnosticOrigin::new("panduck", "semantic"),
        msg.with_fallback(marker.message.clone()),
    )
}

/// Map adapter-layer failures to unified diagnostics.
///
/// Every typed field on each [`AdapterError`] variant is carried into the
/// diagnostic args map so downstream consumers can react on stable keys rather
/// than parsing the human-readable fallback.
pub fn from_adapter_error(error: &AdapterError) -> Diagnostic {
    let origin = DiagnosticOrigin::new("panduck", "adapter");
    match error {
        AdapterError::Io { source, path } => {
            let mut msg = Message::new("panduck.adapter.io")
                .with_arg("source", MessageArg::Text(source.to_string()));
            let fallback = match path {
                Some(path) => {
                    msg = msg.with_arg("path", MessageArg::Text(path.clone()));
                    format!("I/O error at {path}: {source}")
                }
                None => format!("I/O error: {source}"),
            };
            Diagnostic::new(DiagnosticCode::AdapterIo.wire_code(), DiagnosticSeverity::Error, origin, msg.with_fallback(fallback))
        }
        AdapterError::InvalidInput { message } => {
            let diag = Diagnostic::new(
                DiagnosticCode::AdapterInvalidInput.wire_code(),
                DiagnosticSeverity::Error,
                origin,
                Message::new("panduck.adapter.invalid-input")
                    .with_arg("message", MessageArg::Text(message.clone()))
                    .with_fallback(message.clone()),
            );
            diag
        }
        AdapterError::InvalidRange { offset, len } => {
            Diagnostic::new(
                DiagnosticCode::AdapterInvalidRange.wire_code(),
                DiagnosticSeverity::Error,
                origin,
                Message::new("panduck.adapter.invalid-range")
                    .with_arg("offset", MessageArg::U64(*offset as u64))
                    .with_arg("len", MessageArg::U64(*len as u64))
                    .with_fallback(format!("invalid source range at {offset} len {len}")),
            )
        }
        AdapterError::NotImplemented { feature } => Diagnostic::new(
            DiagnosticCode::AdapterNotImplemented.wire_code(),
            DiagnosticSeverity::Error,
            origin,
            Message::new("panduck.adapter.not-implemented")
                .with_arg("feature", MessageArg::Text(feature.clone()))
                .with_fallback(format!("not implemented: {feature}")),
        ),
        AdapterError::Adapter { adapter, message } => Diagnostic::new(
            DiagnosticCode::AdapterFailure.wire_code(),
            DiagnosticSeverity::Error,
            origin,
            Message::new("panduck.adapter.failure")
                .with_arg("adapter", MessageArg::Text(adapter.clone()))
                .with_arg("message", MessageArg::Text(message.clone()))
                .with_fallback(format!("{adapter}: {message}")),
        ),
        AdapterError::UnsupportedFormat { format, operation } => Diagnostic::new(
            DiagnosticCode::AdapterUnsupportedFormat.wire_code(),
            DiagnosticSeverity::Error,
            origin,
            Message::new("panduck.adapter.unsupported-format")
                .with_arg("format", MessageArg::Text(format.clone()))
                .with_arg("operation", MessageArg::Text(operation.clone()))
                .with_fallback(format!("{format} does not support {operation}")),
        ),
        AdapterError::Config { path, message } => {
            let mut msg = Message::new("panduck.adapter.config")
                .with_arg("message", MessageArg::Text(message.clone()));
            let fallback = match path {
                Some(path) => {
                    msg = msg.with_arg("path", MessageArg::Text(path.clone()));
                    format!("config error at {path}: {message}")
                }
                None => format!("config error: {message}"),
            };
            Diagnostic::new(DiagnosticCode::AdapterConfig.wire_code(), DiagnosticSeverity::Error, origin, msg.with_fallback(fallback))
        }
    }
}

/// Collect semantic loss diagnostics from a document graph.
pub fn diagnostics_from_graph(graph: &DocumentGraph) -> DiagnosticSet {
    let mut set = DiagnosticSet::new();
    for marker in &graph.coverage.loss {
        set.push(from_loss_marker(marker));
    }
    set
}

#[cfg(feature = "logger")]
pub use boost_diagnostic::{
    diagnostic_to_log_event, emit_diagnostic as emit_unified_diagnostic, emit_diagnostic_log, emit_diagnostic_set,
};

/// Project a Panduck diagnostic into a log event.
#[cfg(feature = "logger")]
pub fn to_log_event(diagnostic: &Diagnostic) -> boost_log::LogEvent {
    diagnostic_to_log_event(diagnostic)
}

/// Project an adapter failure into a log event.
#[cfg(feature = "logger")]
pub fn adapter_error_to_log_event(error: &AdapterError) -> boost_log::LogEvent {
    diagnostic_to_log_event(&from_adapter_error(error))
}

/// Emit one unified diagnostic through the global logger facade.
#[cfg(feature = "logger")]
pub fn emit_diagnostic(diagnostic: &Diagnostic) {
    emit_unified_diagnostic(diagnostic);
}

/// Emit an adapter failure as a structured diagnostic log event.
#[cfg(feature = "logger")]
pub fn emit_adapter_error(error: &AdapterError) {
    emit_unified_diagnostic(&from_adapter_error(error));
}

/// Emit an adapter failure when the `logger` feature is enabled.
pub fn log_adapter_error(error: &AdapterError) {
    #[cfg(feature = "logger")]
    emit_adapter_error(error);
}

/// Emit every diagnostic in a set when the `logger` feature is enabled.
pub fn log_diagnostic_set(set: &DiagnosticSet) {
    #[cfg(feature = "logger")]
    emit_diagnostic_set(set);
}

/// Build a single-diagnostic envelope for failed adapter operations.
pub fn envelope_from_adapter_error(error: &AdapterError) -> DiagnosticEnvelope {
    let mut set = DiagnosticSet::new();
    set.push(from_adapter_error(error));
    DiagnosticEnvelope::from_set(&set)
}
