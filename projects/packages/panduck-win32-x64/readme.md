# @notedge/panduck-win32-x64

The Windows piece Panduck needs to run fast on 64-bit PCs. You
install [@notedge/panduck](https://www.npmjs.com/package/@notedge/panduck), not this package name.

```bash
npm install -g @notedge/panduck
panduck convert ./draft.docx --to markdown -o ./draft.md
```

npm adds this helper automatically on `win32` + `x64`. If `panduck doctor` fails, make sure you did not run
`npm install --omit=optional`.

License: MPL-2.0
