# panduck-napi

Rust crate that exposes Panduck to Node through N-API. The npm tarballs (`@notedge/panduck` plus per-platform optional
packages) are built from here.

If you are not hacking bindings, install the CLI instead:

```bash
npm install -g @notedge/panduck
panduck doctor
```

Exported to JS today: format discovery, `convertDocument`, `convertMarkdownProject`, DOCX-style `inspectIndex` /
`inspectDecode`, each returning `reportJson` where applicable.

License: MPL-2.0
