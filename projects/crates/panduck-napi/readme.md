# panduck-napi

Node-API bindings for Panduck. Built with `@napi-rs/cli` and published through `@notedge/panduck-<platform>` optional dependencies.

## 📤 Exported operations

| Area | Binding surface |
|------|-----------------|
| Discovery | `supportedFormats`, `supportedConversions`, `supportsConversion` |
| Conversion | `convertDocument(from, to, path)` with `reportJson` |
| Inspection | Container-stage reports for packages like DOCX |

File conversion requires the native `.node` binary for the host OS/CPU.

## 🔧 Build

From the repository root:

```bash
pnpm run build:napi
```

## 🌐 WASM sibling

`panduck-wasm` exposes format discovery only in current releases. Do not expect `convertDocument` from the WASM build.

License: MPL-2.0
