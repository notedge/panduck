# panduck-convert

Rust orchestration layer: read a source through `notedown-formats`, hold `notedown-ir::DocumentGraph`, write a target,
attach `panduck.report/v1`.

`supportsConversion` answers single-file routes. `supports_markdown_project_source` answers `--to markdown-project`.
Listing a format name is weaker than either gate—always plan before batch work.

CLI users never touch this crate. Adapter authors do.

License: MPL-2.0
