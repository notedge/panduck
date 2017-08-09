# Panduck user reference

## npm packages

| Package | Role |
|---------|------|
| `@notedge/panduck` | TypeScript loader and shared types |
| `@notedge/panduck/node` | Node-API entry (`loadPanduckNode`) |
| `@notedge/panduck/wasm` | WebAssembly entry (`loadPanduckWasm`) |
| `@notedge/panduck-<platform>` | Prebuilt native binary for your OS/CPU |
| `@notedge/panduck-unknown-wasm32` | WASM artifacts for browser builds |
| `@notedge/panduck-skills` | Agent skill installer (this package) |

## `PanduckBindings` (current surface)

```ts
type PanduckBindings = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
};
```

Future releases may add `read`, `write`, or `convert` helpers. Agents should read the installed package types (`src/types.ts`) rather than assuming methods that are not exported.

## WASM options

```ts
import { loadPanduckWasm } from "@notedge/panduck/wasm";

await loadPanduckWasm({
    url: "/assets/panduck_wasm_bg.wasm", // optional override
});
```

Use when there is no native `.node` binary (browser, unsupported arch, or sandboxed deploy).

## Format cheat sheet (user expectations)

What users usually want vs what needs extra care:

| From → To | Usually works | Often lossy or manual follow-up |
|-----------|---------------|--------------------------------|
| Markdown → HTML | Headings, lists, links, fenced code | Custom HTML, attributes, MDX |
| RST → HTML / MD | Sections, literals, simple roles | Domain directives, custom roles |
| Org → HTML | Outline, blocks, basic markup | Agenda, citations, Babel blocks |
| LaTeX → other | Plain text fragments | Full math + macro-heavy TeX |
| Word / PDF → text | — | Layout, styles, reading order (check roadmap) |

Tell the user which row applies before running a long batch.

## Semantic loss (plain language)

When describing results to users, use:

- **Preserved** — heading level, link URL, code block language, image reference
- **Inferred** — table alignment guessed, PDF column order reconstructed
- **Unsupported** — feature has no target equivalent (e.g. custom Word style → plain paragraph)
- **Dropped** — element omitted from output; must be called out, never silent

## Minimal probe script

Useful to verify install before a batch job:

```ts
import { loadPanduckNode } from "@notedge/panduck/node";

const p = loadPanduckNode();
console.log("version", p.panduckVersion());
console.log("formats", p.supportedFormats().join(", "));
```

## Links

- Repository: https://github.com/oovm/panduck
- Product loader: `@notedge/panduck` on npm (when published)
