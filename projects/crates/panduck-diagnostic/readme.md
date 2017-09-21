# panduck-diagnostic

Panduck domain codes and builders on top of the shared `diagnostic` crate.

Maps `AdapterError` and `notedown-ir` loss markers into unified diagnostics for CLI and `panduck.report/v1` output. Syntax and container facts remain owned by Oak and Acorn.

With the `console` feature, use `emit_adapter_error`, `emit_diagnostic`, and `emit_diagnostic_set` to project diagnostics into the shared `console` facade via `diagnostic_to_event`.
