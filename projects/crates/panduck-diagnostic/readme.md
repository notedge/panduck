# panduck-diagnostic

Panduck domain codes and builders on top of the shared `diagnostic` crate.

Maps `AdapterError` and `notedown-ir` loss markers into unified diagnostics for CLI and `panduck.report/v1` output. Syntax and container facts remain owned by Oak and Acorn.

With the `logger` feature, use `emit_adapter_error`, `emit_diagnostic`, and `emit_diagnostic_set` to project diagnostics into the shared `logger` facade via `diagnostic_to_log_event`.

The `console` feature is a deprecated alias for `logger` and remains only for transitional callers.
