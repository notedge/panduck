# panduck-types

Shared conversion contracts for Panduck adapters. Bridges format-specific readers/writers with `notedown-ir` without duplicating document semantics.

## 📋 Responsibilities

| Item | Role |
|------|------|
| `AdapterError` / `Result` | Fatal adapter failures |
| `SourceText`, `BinaryReader`, `TextWriter` | IO boundaries for converters |
| `notedown_ir` re-export | Canonical `DocumentGraph` types |

Syntax diagnostics belong to Oak. Container diagnostics belong to Acorn. Semantic loss belongs to `notedown_ir::CoverageReport`.

## 🔌 Integration

```rust
use panduck_types::{AdapterError, Result};

fn read_bytes() -> Result<Vec<u8>> {
    Err(AdapterError::not_implemented("example reader"))
}
```

`panduck-convert` and `panduck-napi` depend on these types for stable error surfaces across CLI and Node.

License: MPL-2.0
