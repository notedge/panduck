# panduck

Single entry point for the Panduck document conversion family. Each enabled feature opens a `panduck::…` module that re-exports one workspace crate — same types, no duplicate wrappers.

Product overview: [repository README](../../../readme.md).

## Features

Default features are **empty**. Enable only what you need:

| Feature | Module | Underlying crate |
|---------|--------|------------------|
| `types` | `panduck::types` | `panduck-types` |
| `diagnostic` | `panduck::diagnostic` | `panduck-diagnostic` |
| `convert` | `panduck::convert` | `panduck-convert` |
| `logger` | `panduck::convert` + `panduck::diagnostic` | structured log projection |

`diagnostic` and `convert` also enable `types`.

## Example

```toml
[dependencies]
panduck = { version = "0.0.4", default-features = false, features = ["convert", "logger"] }
```

```rust
use panduck::convert::convert_bytes;
use panduck::types::AdapterError;
```
