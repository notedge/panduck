---
name: panduck
description: Work on Panduck document conversion. Treat notedown-ir DocumentGraph as the semantic core. Readers consume Oak or Acorn views and lower to notedown-ir; writers consume notedown-ir only. Respect Acorn, Oak, and notedown-ir diagnostics layers. Use thin panduck_types::AdapterError for fatal adapter failures only. Do not invent Panduck-owned document ASTs or restore Gaia-style diagnostics containers.
---

# Panduck for agents

## One sentence

**Panduck = format conversion adapters around `notedown-ir::DocumentGraph`, not a fourth document AST.**

## Four-system boundary

```text
Acorn     binary layout, containers, random access, compression, provenance
Oak       text syntax CST / typed AST (markdown, rst, xml, notedown markup, …)
Notedown  notedown-ir — semantic graph, relations, assets, source, coverage
Panduck   readers (into IR), writers (from IR), filters, CLI / N-API orchestration
```

These are not interchangeable ASTs. Every hop needs an explicit contract and loss record.

## Conversion hub

```text
source bytes or text
  → Acorn (if binary package) or direct text
  → Oak lexer/parser (if textual)
  → format adapter (WordprocessingML, HTML, … when needed)
  → notedown-ir::DocumentGraph
  → Panduck writer
  → target bytes or text
```

Rules:

1. **Readers** construct `DocumentGraph` directly. No temporary Notedown markup text. No `notedown_ast` / HIR as the hub.
2. **Writers** read `DocumentGraph` and target policy only. No parser AST parameters.
3. **`panduck-types`** holds conversion contracts, thin adapter errors, binary helpers. It re-exports `notedown_ir` but does **not** own block/inline semantics.
4. Legacy `ast/` modules under `panduck-markdown` / `panduck-rst` are transitional. New work lowers Oak → `notedown-ir`, not into Panduck AST.

## Crate map

| Crate / package | Role |
|-----------------|------|
| `notedown-ir` | `DocumentGraph`, `Block`, `Inline`, `Relation`, `Asset`, `SourceRef`, `LossMarker`, `CoverageReport` |
| `panduck-types` | Shared contracts, adapter I/O helpers, `pub use notedown_ir` |
| `panduck-markdown` | Markdown reader/writer; `oak::` re-exports `oak-markdown`; `ir::` re-exports notedown-ir |
| `panduck-rst` | RST reader/writer; `oak::` re-exports `oak-rst`; `ir::` re-exports notedown-ir |
| `panduck-org`, `panduck-tex`, … | Other format adapters (same IR hub) |
| `panduck-napi` / `panduck-wasm` | Native bindings |
| `@notedge/panduck` | TypeScript / Node-API / WASM product surface |

Forbidden names: `panduck-core` (use `panduck-types`).

## Diagnostics and loss (by layer)

Do **not** reintroduce `PanduckDiagnostics`, `PanduckError`, or Gaia leftovers (`Architecture`, `CompilationTarget`, `InvalidInstruction`, …).

| Failure | Owner |
|---------|--------|
| Container / ZIP / OPC / partial read | Acorn diagnostic |
| Text syntax / XML well-formedness | Oak diagnostic |
| Semantic gap, unresolved ref, inferred layout | `notedown-ir` `SemanticStatus`, `LossMarker`, `coverage` |
| Writer cannot represent a construct | IR `LossMarker` + optional `AdapterError::adapter` |
| I/O, config, unsupported target format | `panduck_types::AdapterError` (`Result<T>`) |

**Success with loss is normal.** A written file does not mean semantics were preserved. Always surface `graph.coverage` and explicit loss markers when returning results to users or tests.

## Dependency conventions (panduck monorepo)

- **Internal crates**: `path` in root `Cargo.toml` `[workspace.dependencies]`.
- **External siblings** (`oaks`, `acorn.rs`, `notedown`): `git` + `branch = "dev"` in committed `Cargo.toml`.
- **Local overlay**: copy `.cargo/config.toml.example` → `.cargo/config.toml` (gitignored) to patch sibling paths.
- Do **not** patch internal `panduck-*` crates via `.cargo/config.toml`.
- Oak crates may require **nightly** (`rust-toolchain.toml`).

## Agent workflow

### Before editing

1. Identify which boundary you touch: Acorn, Oak, adapter, IR, or writer.
2. Read the relevant adapter crate and `notedown-ir` types you will populate.
3. If the DXO workspace is available, read design truth in `规划设计/acorn/13-Oak-Notedown-Panduck边界.md` and `规划设计/notedown/00-文档语义IR独立合同.md`. Do not cite those internal paths in committed Rust comments.

### Implementing a reader

1. Parse with `oak-*` (or decode with `acorn-*` then Oak).
2. Walk typed AST; map to `Block`, `Inline`, `Relation`, `Asset`, `SourceRef`.
3. Allocate stable ids via `IdAllocator` / `DocumentGraph` APIs.
4. Record provenance and `SemanticStatus` (resolved, inferred, partial, unsupported, lossy).
5. Return `DocumentGraph` (and adapter-level `Result` for fatal failures only).

### Implementing a writer

1. Declare capability: footnotes, bidirectional links, assets, math, styles, pagination.
2. Traverse IR; emit target format.
3. For unsupported nodes, append `LossMarker` rather than silently dropping.
4. Never require Notedown text or Oak AST at write time.

### Tests

- Prefer integration tests that round-trip or compare IR snapshots.
- Assert coverage / loss when exercising partial formats (PDF infer, DOCX style gaps).
- Do not add tests that only check legacy Panduck AST shapes.

## npm / bindings

- Product package: `@notedge/panduck` (`projects/packages/panduck`).
- Platform optional deps: `@notedge/panduck-<platform>`, `@notedge/panduck-unknown-wasm32`.
- Homepage probe site: `@notedge/homepage` (VMZ).

## Repository hygiene

- No `docs/` directory in the panduck implementation repo.
- No parallel root `AGENTS.md` — this skill is the agent entry for Panduck.
- Design decisions belong in the external `规划设计/` tree when working inside the DXO workspace, not only in chat.
- Git commits: gitmoji subject, UTF-8 via Python + `git commit -F` on Windows, no internal milestone codes in messages.

## Default user prompts

Users state the document goal. They do not need to repeat architecture rules.

```text
Convert this Markdown to HTML with Panduck and list any semantic loss.
```

```text
Implement oak-markdown → notedown-ir lowering in panduck-markdown.
```

```text
Add a writer from notedown-ir to static HTML with footnotes and asset links.
```

## Boundaries (never do)

- Do not add a Panduck-owned document AST as the long-term model.
- Do not stringify through Notedown markup as an IR interchange format.
- Do not fold Oak syntax errors into Panduck Gaia diagnostics.
- Do not invent APIs absent from `notedown-ir` or the target adapter contract.
- Do not use `git add -A` or recursive delete without explicit user authorization (SEV-0).

## More detail

See [reference.md](reference.md) for IR object families and asset identity rules.
