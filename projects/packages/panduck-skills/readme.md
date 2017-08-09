# @notedge/panduck-skills

Agent skills for **people who use Panduck** to convert documents — not for contributors hacking the Rust repo.

Install once so coding agents know how to install `@notedge/panduck`, pick formats, write conversion scripts, and explain partial or failed conversions in plain language.

## Install

```bash
npx @notedge/panduck-skills
```

```bash
npx @notedge/panduck-skills -g
npx @notedge/panduck-skills -a cursor -y
```

## Example prompts

```text
Convert this folder of Markdown files to HTML with Panduck. List anything that did not round-trip.
```

```text
Write a Node script using @notedge/panduck that checks which formats are available on this machine.
```

```text
I need RST → Markdown for my docs site. Set up Panduck and handle images in a subfolder.
```

## What the skill covers

- Installing and loading `@notedge/panduck` (Node native vs WASM)
- Supported format names and honest limits of the current API
- Batch conversion, CI, and troubleshooting platform packages
- Explaining semantic loss to end users without internal jargon

See `skills/panduck/SKILL.md` for the full agent guide.
