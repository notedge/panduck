# @notedge/panduck-unknown-wasm32

WebAssembly build of Panduck for **browser and Worker** hosts. `@notedge/panduck` depends on this package for the `@notedge/panduck/wasm` entry point.

## 📦 Role in the stack

```bash
npm install @notedge/panduck
```

```ts
import { loadPanduckWasm } from "@notedge/panduck/wasm";

const panduck = await loadPanduckWasm();
console.log(panduck.panduckVersion?.());
console.log(panduck.supportedFormats?.());
```

**This release:** format discovery and version metadata only. There is no `convertDocument` on the WASM surface. Run file conversion through `@notedge/panduck/node` on Node 20+.

## 🛠 Build from source

Clone https://github.com/notedge/panduck and populate `lib/`:

```bash
pnpm install
pnpm run build:wasm
```

Published tarballs include prebuilt `lib/panduck_wasm.js` and the `.wasm` artifact.

## 🔧 Troubleshooting

| Symptom | Check |
|---------|-------|
| WASM loads but convert is missing | Expected—use Node native binding for conversion |
| `lib/` empty in a git checkout | Run `pnpm run build:wasm` before importing locally |

Main package: https://www.npmjs.com/package/@notedge/panduck

License: MPL-2.0
