# panduck-types

Shared conversion contracts for Panduck adapters.

Document semantics live in `notedown_ir::DocumentGraph`. This crate provides:

- `AdapterError` and `Result<T>` for fatal adapter failures
- `SourceText`, `BinaryReader`, `TextWriter`
- re-export of `notedown_ir`

Syntax diagnostics belong to Oak. Container diagnostics belong to Acorn. Semantic loss belongs to
`notedown_ir::CoverageReport`.

## Example

```rust
use panduck_types::{AdapterError, Result};

fn read_bytes() -> Result<Vec<u8>> {
    Err(AdapterError::not_implemented("example reader"))
}
```
