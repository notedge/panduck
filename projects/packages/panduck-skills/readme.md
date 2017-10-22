# @notedge/panduck-skills

Agent skill pack for **people who use Panduck** to convert documents—not for contributors hacking the Rust repository.

Install once so coding agents know how to install `@notedge/panduck`, verify a conversion route, write outputs safely, and explain partial or failed conversions in plain language.

**Scope:** instructions only. The installer does not add conversion routes, platform binaries, or WASM file conversion.

## 📥 Install

```bash
npx @notedge/panduck-skills
```

```bash
npx @notedge/panduck-skills -g
npx @notedge/panduck-skills -a cursor -y
```

## 💬 Example prompts

```text
Use Panduck to check whether ./notes/paper.docx can convert to Markdown.
If supported, write ./notes/paper.md and ./notes/paper.report.json without modifying the source.
List semantic losses from the report.
```

```text
Write a Node script with @notedge/panduck that calls supportsConversion for docx→markdown
on this machine and converts one sample file.
```

```text
I have a folder of Markdown files. Plan panduck batch conversion to docx and flag any
unsupported paths before running.
```

## ✅ What the agent checks

- `panduck doctor` and native binding load
- `plan`, `formats`, or `supportsConversion` before promising a target format
- Separate read vs write capability—registered names are not full matrices
- New output paths and `--report` for `panduck.report/v1`
- Asset handling (embedded vs linked vs omitted) from report losses
- WASM vs Node: discovery in browser, conversion on Node for this release

## 📎 Full skill

See [`skills/panduck/SKILL.md`](skills/panduck/SKILL.md) for the complete workflow.

Install the runtime separately:

```bash
npm install @notedge/panduck
```

License: MPL-2.0
