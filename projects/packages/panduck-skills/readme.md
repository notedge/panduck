# @notedge/panduck-skills

[![npm version](https://img.shields.io/npm/v/@notedge/panduck-skills.svg)](https://www.npmjs.com/package/@notedge/panduck-skills) [![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](https://www.mozilla.org/MPL/2.0/) [![Node.js](https://img.shields.io/badge/Node.js-%3E%3D20-339933)](https://nodejs.org/)

Agent skill pack for **people who use Panduck** with coding agents—not for contributors hacking the Rust repository.

Install once so agents know how to install `@notedge/panduck`, verify a conversion route, write outputs safely, and explain partial or failed conversions in plain language.

**Scope:** instructions only. The installer does not add conversion routes, platform binaries, or WASM file conversion.

```bash
npm install -g @notedge/panduck-skills
panduck-skills -y
```

Pass `-a <agent>` when your skills CLI requires a host name.

Install Panduck too:

```bash
npm install -g @notedge/panduck
panduck doctor
```

Example prompt:

```text
Can ./notes/paper.docx become Markdown with panduck?
If yes, write ./notes/paper.md and a report file next to it. Do not edit the original.
Tell me what the report says was lost.
```

```text
Turn ./report.docx into a markdown-project folder at ./report-md and summarize missing images from panduck.report.json.
```

Full skill text: [`skills/panduck/SKILL.md`](skills/panduck/SKILL.md)

License: MPL-2.0
