# @notedge/panduck-unknown-wasm32

Lets a web page ask Panduck which document formats exist. It does **not** convert a `.docx` on your hard drive—that
still needs the normal install:

```bash
npm install -g @notedge/panduck
panduck convert ./draft.docx --to markdown -o ./draft.md
```

In the browser:

```ts
import { loadPanduckWasm } from "@notedge/panduck/wasm";
const panduck = await loadPanduckWasm();
panduck.supportedFormats?.();
```

Usually pulled in automatically when you depend on `@notedge/panduck`.

License: MPL-2.0
