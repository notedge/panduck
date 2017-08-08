# Panduck agent reference

## notedown-ir object families

Minimum semantic objects (independent of any parser AST):

- Document metadata: title, language, authors, tags.
- Blocks: section, paragraph, list, table, quote, code, math, opaque (with status).
- Inlines: text runs, emphasis, code, links, math, references.
- Relations: contains, references, bidirectional, cites, embeds, backlink (indexed, not injected into target body).
- Assets: image, audio, video, font, attachment, generated view — with `AssetId`, optional content identity, media type, status.
- Sources: `SourceRef` per node — XML range, package member, PDF object, text span, synthetic, manual; precision via `SemanticStatus`.

## Asset three-layer identity

```text
AssetId        stable document-level identity
AssetSource    package member / PDF object / external URI
AssetView      decoded, thumbnail, or target-specific rendition
```

Writers remap paths and relationships per target format. Do not collapse source, normalized content, and output path into one string.

## DOCX path (reference)

```text
DOCX bytes
  → Acorn ZIP / OPC
  → member Decoded view
  → Oak XML AST
  → WordprocessingML adapter
  → notedown-ir::DocumentGraph
  → Panduck writer (HTML, EPUB, …)
```

`oak-xml` owns XML syntax. Namespace, mixed content, and whitespace need explicit adapter logic — not string search.

## EPUB / HTML / PDF notes

- **HTML**: Oak HTML AST → adapter → IR; preserve link targets and media as relations + assets.
- **EPUB**: Acorn container + multiple Oak XML/HTML members → IR with package-level asset table.
- **PDF**: layout is observational; inferred reading order stays `Inferred` / `Partial`; math must not downgrade to code blocks.

## Local development patch template

Committed: `panduck/.cargo/config.toml.example`

```toml
[patch."https://github.com/yggdrasil-language/oaks.git"]
oak-markdown = { path = "../oaks/examples/oak-markdown" }

[patch."https://github.com/notedge/notedown.git"]
notedown-ir = { path = "../notedown/projects/crates/notedown-ir" }
```

Copy to `.cargo/config.toml` (gitignored) when sibling repos exist.

## Useful entry files

| Path | Purpose |
|------|---------|
| `projects/crates/panduck-types/src/lib.rs` | Contracts + `notedown_ir` re-export |
| `projects/crates/panduck-markdown/src/lib.rs` | `oak::`, `ir::` modules |
| `projects/packages/panduck/src/index.ts` | TS product entry |
| `notedown/projects/crates/notedown-ir/src/graph.rs` | `DocumentGraph` API |
