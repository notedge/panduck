# @notedge/panduck

Node 20+ package for Panduck: CLI commands, `loadPanduckNode()`, and WASM format discovery. Use it when you need to convert a local file between supported formats and read a `panduck.report/v1` JSON result.

File conversion runs through the native binding on Node. The WASM export lists formats and version metadata only—do not expect `convertDocument` in the browser build for this release.

## 🤖 Agent instructions

`@notedge/panduck-skills` teaches coding agents to check routes before converting. It installs instructions, not this runtime.

```bash
npx @notedge/panduck-skills
```

## 📦 Install

```bash
npm install @notedge/panduck
npx panduck doctor
```

Platform `.node` binaries install through optional dependencies (`@notedge/panduck-win32-x64`, `@notedge/panduck-linux-x64`, `@notedge/panduck-darwin-arm64`, and siblings).

## 🔄 Convert one file

Always write to a **new** path and keep the source unchanged:

```bash
npx panduck plan ./draft.docx --to markdown
npx panduck convert ./draft.docx --to markdown -o ./draft.md --report ./draft.report.json
```

Check what the binding actually supports:

```bash
npx panduck formats --json
```

## 🔌 Node API

```ts
import { loadPanduckNode } from "@notedge/panduck/node";

const panduck = loadPanduckNode();
if (!panduck.supportsConversion?.("docx", "markdown")) {
  throw new Error("route not available on this build");
}
const result = panduck.convertDocument!("docx", "markdown", "./draft.docx");
console.log(result.reportJson);
```

Exports:

- `@notedge/panduck` — package metadata
- `@notedge/panduck/node` — `loadPanduckNode()`
- `@notedge/panduck/wasm` — format discovery (no file conversion here)
- `@notedge/panduck/cli` — programmatic CLI builder

## 📊 Read the report

Open `panduck.report/v1` JSON for `status`, `coverage`, `losses`, `diagnostics`, and `outputs`. A successful exit can still be `success_with_loss`—read losses before publishing converted content.

`panduck inspect` helps audit container structure (for example DOCX package index) without promising a full semantic read.

## ✅ Verified routes (native)

When `supportsConversion(from, to)` is true:

| From | To |
|------|-----|
| `docx` | `markdown`, `docx` |
| `markdown` | `markdown`, `docx` |
| `notedown` | `markdown` |
| `epub` | `markdown` |

Registered format names beyond these pairs may read or list without a working writer. Plan before batch jobs.

## 🔧 Troubleshooting

| Symptom | Check |
|---------|-------|
| `Unsupported platform for Panduck native bindings` | OS/CPU mismatch or missing optional platform package |
| `conversion pipeline is not wired yet` | Route not implemented—use `plan` / `supportsConversion` |
| Output exists but quality is wrong | Report `losses` for tables, footnotes, images |
| Browser bundle cannot convert | Use Node native path for file conversion |

Repository: https://github.com/notedge/panduck

License: MPL-2.0
