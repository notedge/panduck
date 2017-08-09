---
name: panduck
description: Help users convert documents with Panduck (@notedge/panduck). Install Node or WASM bindings, pick source and target formats, run conversions in scripts or apps, and explain partial results or semantic loss in plain language. Load when the user mentions Panduck, document conversion, Markdown, reStructuredText, Org, LaTeX, DOCX, EPUB, or HTML export.
---

# Panduck for users

## One sentence

**Panduck converts documents between formats while surfacing what was preserved, inferred, or lost — not a silent copy-paste.**

## When to use this skill

The user has a **document job**, not a Rust monorepo task. Typical goals:

- Convert a file or folder from format A to format B
- Batch-convert exports (notes, docs, wikis, manuscripts)
- Wire Panduck into Node, TypeScript, or a static site
- Compare outputs and understand missing footnotes, math, images, or styles
- Fix install errors for `@notedge/panduck` platform packages

Do **not** default to internal crate names, IR types, or contributor workflows unless the user explicitly asks to hack on the Panduck repository.

## Install

**npm package:** `@notedge/panduck`

```bash
npm install @notedge/panduck
# or
pnpm add @notedge/panduck
```

Native speed on Node uses an optional platform package (`@notedge/panduck-win32-x64`, `@notedge/panduck-darwin-arm64`, …) pulled in automatically when supported.

**Browser / edge:** use the WASM entry (`@notedge/panduck/wasm`) when you cannot load a `.node` binary.

**This skill package** (teaches agents how to help Panduck users):

```bash
npx @notedge/panduck-skills
npx @notedge/panduck-skills -a cursor -y
```

## Quick start (Node)

```ts
import { loadPanduckNode } from "@notedge/panduck/node";

const panduck = loadPanduckNode();
console.log(panduck.panduckVersion());
console.log(panduck.supportedFormats());
console.log(panduck.isSupportedFormat("markdown"));
```

Build native artifacts from source only when the user is developing Panduck itself: `pnpm run build:napi` at the repo root.

## Supported formats (today)

Bindings currently advertise these adapter names:

| Format | Name passed to `isSupportedFormat` |
|--------|-------------------------------------|
| Markdown | `markdown` |
| reStructuredText | `rst` |
| Org mode | `org` |
| LaTeX | `tex` |

Roadmap formats (DOCX, EPUB, HTML, PDF, Notedown) may appear in docs or issues before they are callable from npm. **Always call `supportedFormats()`** and tell the user honestly if a path is not available yet.

## How to help the user

### 1. Clarify the job

Ask only what affects the conversion:

- Source path(s) or pasted content
- Desired output format and encoding (UTF-8, LF line endings)
- Must-keep features: footnotes, citations, math, tables, images, internal links
- One-off file vs batch / watch folder / CI step
- Runtime: Node script, server, or browser

### 2. Pick the binding

| Environment | Import |
|-------------|--------|
| Node / Bun / server | `@notedge/panduck/node` → `loadPanduckNode()` |
| Cached singleton | `loadPanduckNative()` from `@notedge/panduck` |
| Browser / WASM | `@notedge/panduck/wasm` → `loadPanduckWasm()` |

### 3. Run conversion

Use the **public API** exposed on `PanduckBindings`. If a convert/read/write helper is not on the binding yet:

- Say so clearly
- Offer a practical workaround (e.g. export to an intermediate format the user already has)
- Do not invent hidden Rust APIs or tell the user to patch `Cargo.toml`

When conversion APIs exist, prefer them over shelling out to random third-party CLIs unless the user asks for a specific tool.

### 4. Report results honestly

Users care about **outcomes**, not internal IR names.

Always mention when relevant:

- **Full success** — structure and media match expectations
- **Partial** — file was written but some constructs were dropped, flattened, or guessed (e.g. complex tables, custom styles, PDF reading order)
- **Failed** — unsupported format, corrupt input, or missing platform binary

If the API returns coverage or loss metadata, summarize it in a short bullet list. If not, diff headings, link targets, and image references against the source.

## Example user prompts

```text
Convert notes/*.md to static HTML with Panduck. Keep footnotes and flag anything that did not round-trip.
```

```text
I installed @notedge/panduck on Windows and get "Unsupported platform". What should I install?
```

```text
Batch convert these .rst files to Markdown for my wiki. Use a Node script I can run in CI.
```

```text
Does Panduck support DOCX yet? If not, what is the closest path from Word to Markdown?
```

## Troubleshooting

| Symptom | What to check |
|---------|----------------|
| `Unsupported platform for Panduck native bindings` | OS/arch not in optional platform packages; try WASM or another machine |
| Empty or stub output | Format may be listed but conversion not fully implemented — verify with a minimal sample |
| Missing images | Relative asset paths; copy `media/` alongside output or rewrite URLs in post-processing |
| Math looks wrong | Source dialect ( `$...$` vs `$$...$$`, RST roles) may not map 1:1 — show source and output snippet |

## Agent discipline

- **User-first language** — "your Markdown file", "the HTML export", not `panduck-markdown` or `DocumentGraph`.
- **Check the binding** — `supportedFormats()` before promising a format.
- **No fake APIs** — only document methods on `PanduckBindings` and published package exports.
- **Preserve user files** — copy or write to a new path unless they ask to overwrite.
- **Batch safety** — dry-run on one file, then scale; log per-file status.

## More detail

See [reference.md](reference.md) for binding fields, WASM options, and a format capability cheat sheet.
