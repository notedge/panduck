# @notedge/panduck-skills

Agent skills for [Panduck](https://github.com/oovm/panduck) document conversion.

Install once. The skill teaches agents the Acorn / Oak / Notedown IR / Panduck boundaries, crate layout, and conversion workflow so prompts can focus on the document task.

## Install

```bash
npx @notedge/panduck-skills
```

Forward flags to the skills CLI:

```bash
npx @notedge/panduck-skills -g
npx @notedge/panduck-skills -a cursor -y
```

## Example prompts

```text
Convert this Markdown file to HTML with Panduck. Report any semantic loss from notedown-ir coverage.
```

```text
Add a Panduck reader: oak-markdown → notedown-ir. Do not add a Panduck-owned document AST.
```

```text
Wire a DOCX import path: Acorn OPC → Oak XML → adapter → notedown-ir.
```

## What the skill covers

- Four-system boundary: Acorn, Oak, Notedown IR, Panduck
- `notedown-ir::DocumentGraph` as the semantic hub
- Per-crate roles (`panduck-types`, `panduck-markdown`, …)
- Diagnostics and loss reporting by layer
- Monorepo dependency and local patch conventions

See `skills/panduck/SKILL.md` for the full agent contract.
