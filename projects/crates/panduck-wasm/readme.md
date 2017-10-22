# panduck-wasm

WebAssembly bindings for Panduck. Packaged for npm as `@notedge/panduck-unknown-wasm32` and loaded through `@notedge/panduck/wasm`.

## ✅ This release

- Version metadata
- `supportedFormats` / format discovery helpers

## 🚫 Not on WASM surface

`convertDocument` and file-path conversion are **not** exported to browser hosts in the current binding. Node native (`panduck-napi`) performs file conversion.

## 🛠 Build

```bash
pnpm run build:wasm
```

Output lands in `projects/packages/panduck-unknown-wasm32/lib/` for npm publishing.

License: MPL-2.0
