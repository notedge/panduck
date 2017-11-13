# @notedge/panduck

[![npm version](https://img.shields.io/npm/v/@notedge/panduck.svg)](https://www.npmjs.com/package/@notedge/panduck) [![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](https://www.mozilla.org/MPL/2.0/) [![Node.js](https://img.shields.io/badge/Node.js-%3E%3D20-339933)](https://nodejs.org/)

Panduck turns Word files, Markdown, EPUB, and related documents into another format—and shows you what got lost along
the way. Footnotes, tables, images, and fancy Word styles often come through only partly; you get a JSON report saying
what happened instead of a silent bad export.

You can output a single file, or a folder ready for git: `index.md` (or `chapters/*.md`), copied images under `assets/`,
and `panduck.report.json`.

## Install

Put the `panduck` command on your PATH:

```bash
npm install -g @notedge/panduck
panduck doctor
```

If `doctor` prints an error, Panduck did not load on your OS. On Windows, Linux, and macOS the right native helper
installs automatically with `@notedge/panduck`—you normally do not install `@notedge/panduck-win32-x64` and friends by
hand.

Prefer a one-off run? `npx @notedge/panduck …` works without `-g`.

## Using the CLI

**See which formats Panduck knows about on your computer:**

```bash
panduck formats --json
```

**Check one file before a big batch** (will this docx become markdown without surprises?):

```bash
panduck plan ./draft.docx --to markdown
```

**Word → Markdown**, writing new files and leaving the original alone:

```bash
panduck convert ./draft.docx --to markdown -o ./draft.md --report ./draft.report.json
```

**EPUB or Word → a whole Markdown project** (text + images in `assets/`):

```bash
panduck convert ./book.epub --to markdown-project -o ./book-md
```

**Dry-run validation** (no output file):

```bash
panduck check ./draft.docx --format markdown
```

**Look inside a DOCX package** (parts and paths, not full body semantics):

```bash
panduck inspect ./draft.docx --stage index --json
```

Handy options: `--strict` fails when anything was lost; `--overwrite` replaces an existing output; `--batch` with
`--output-dir` for many files at once. Panduck will not write to the same path as the input.

## What converts reliably today

These paths are the ones to count on right now:

| You have       | You want              |
|----------------|-----------------------|
| Word (`.docx`) | Markdown              |
| Word (`.docx`) | Word (normalized)     |
| Markdown       | Markdown (cleaned up) |
| Markdown       | Word                  |
| Notedown       | Markdown              |
| EPUB           | Markdown              |

**Markdown project** (folder with `assets/`) works from: Word, legacy `.doc`, PDF, EPUB, HTML, Markdown, and Notedown:

```bash
panduck convert ./paper.docx --to markdown-project -o ./paper-md
```

The `formats` command may list other names (Org, TeX, HTML export, and so on). Seeing a name there does not mean every
direction works—run `plan` on a real file before you script a pipeline.

## The report file

Pass `--report ./something.report.json` or read `panduck.report.json` inside a markdown project folder.

Worth reading before you publish the converted files:

- Did it finish, or finish with losses? (`status`)
- What was dropped or guessed? (`losses`)
- What files were actually written? (`outputs`)

Exit code `0` can still mean content was lost. Open the report.

## Use from Node

```ts
import {loadPanduckNode} from "@notedge/panduck/node";

const panduck = loadPanduckNode();
const {reportJson} = panduck.convertDocument!("docx", "markdown", "./draft.docx");
```

Markdown project from code:

```ts
panduck.convertMarkdownProject!("epub", "./book.epub", "./book-md");
```

Subpaths: `@notedge/panduck/node` (convert files), `@notedge/panduck/wasm` (browser: list formats only, no file
conversion yet), `@notedge/panduck/cli` (run the CLI from code).

## Coding agents

`@notedge/panduck-skills` teaches coding agents how to run Panduck safely. It does not install Panduck itself.

```bash
npm install -g @notedge/panduck-skills
panduck-skills -y
```

## When things go wrong

| You see                                            | Try                                                        |
|----------------------------------------------------|------------------------------------------------------------|
| `Unsupported platform for Panduck native bindings` | Different machine, or reinstall without `--omit=optional`  |
| `conversion pipeline is not wired yet`             | That conversion is not implemented—run `plan` on your file |
| Markdown looks wrong but command succeeded         | Open the report; tables and footnotes are often partial    |
| Images missing in a markdown project               | Check the report for `unresolved` assets                   |
| Browser import has no `convertDocument`            | Use Node; WASM only lists formats for now                  |

More background: https://github.com/notedge/panduck

License: MPL-2.0
