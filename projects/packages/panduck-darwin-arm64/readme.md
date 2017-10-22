# @notedge/panduck-darwin-arm64

Prebuilt Node-API binary for **macOS arm64 (Apple Silicon)**. `@notedge/panduck` loads this package automatically on `darwin` + `arm64` through an optional dependency.

## 📦 Install

```bash
npm install @notedge/panduck
```

npm pulls `@notedge/panduck-darwin-arm64` when the host matches `os: darwin` and `cpu: arm64`.

## 🔧 Mismatch diagnosis

| Symptom | Likely cause |
|---------|----------------|
| `Unsupported platform for Panduck native bindings` | Wrong OS/CPU or optional dependency not installed |
| Binding loads but convert fails | Run `npx panduck doctor` and check `supportsConversion` |

Main package: https://www.npmjs.com/package/@notedge/panduck

License: MPL-2.0
