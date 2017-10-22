# @notedge/panduck-win32-x64

Prebuilt Node-API binary for **Windows x64**. `@notedge/panduck` loads this package automatically on `win32` + `x64` through an optional dependency.

## 📦 Install

```bash
npm install @notedge/panduck
```

npm pulls `@notedge/panduck-win32-x64` when the host matches `os: win32` and `cpu: x64`.

## 🔧 Mismatch diagnosis

| Symptom | Likely cause |
|---------|----------------|
| `Unsupported platform for Panduck native bindings` | Wrong OS/CPU or optional dependency not installed |
| Binding loads but convert fails | Run `npx panduck doctor` and check `supportsConversion` |

Main package: https://www.npmjs.com/package/@notedge/panduck

License: MPL-2.0
