# panduck-diagnostic

Domain diagnostic codes and builders for Panduck on top of the shared `diagnostic` crate.

Maps `AdapterError` and `notedown-ir` loss markers into unified diagnostics for CLI output and `panduck.report/v1` JSON.

## 🧭 Ownership boundaries

| Layer | Owns |
|-------|------|
| Oak | Syntax diagnostics |
| Acorn | Container/package diagnostics |
| `notedown-ir` | Semantic coverage and loss |
| This crate | Panduck-specific projection into reports and logs |

## 🔌 Logger feature

With the `logger` feature enabled, use `emit_adapter_error`, `emit_diagnostic`, and `emit_diagnostic_set` to project diagnostics into the shared `logger` facade via `diagnostic_to_log_event`.

The `console` feature is a deprecated alias for `logger`.

## 🔌 Integration

`panduck-convert` and the CLI pipeline call into this crate when building `panduck.report/v1`. End users read the JSON report—not this crate API.

License: MPL-2.0
