# Adapter errors

Adapter-layer errors for Panduck conversion.

Syntax and container diagnostics belong to Oak and Acorn. Semantic loss belongs to `notedown_ir::CoverageReport`.

## AdapterError

Fatal failure in a Panduck reader, writer, or orchestration step. Variants cover I/O, invalid input, unsupported
formats, and missing adapter capabilities.

## Result

Panduck adapter result type alias: `std::result::Result<T, AdapterError>`.
