# panduck-convert

Conversion routes over `notedown-ir::DocumentGraph` for Panduck. Orchestrates readers and writers registered for each supported format pair and emits `panduck.report/v1` coverage data.

End users should call `@notedge/panduck` CLI or `loadPanduckNode()`—not this crate directly unless building a new adapter.

## 🔄 Route model

Each route implements read → graph → write for a verified `(from, to)` pair. `supportsConversion` gates whether a route is callable in the current build.

Registered format **names** can exist without a working writer. Listing `html` in `supportedFormats()` does not prove HTML export works.

## 📊 Reports

Semantic loss is recorded in the graph coverage report and projected into `panduck.report/v1` JSON by `panduck-diagnostic`. Syntax issues stay with Oak; container issues with Acorn.

## ✅ Native routes in current releases

See `@notedge/panduck` readme for the verified matrix (`docx`↔`markdown`, `epub`→`markdown`, `notedown`→`markdown`, etc.). Plan or probe before batch conversion.

License: MPL-2.0
