// wasm-pack writes `lib/.gitignore` containing `*`, which makes `npm pack` drop
// every generated artifact. Remove it so the wasm package ships its payload.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const targets = ["projects/panduck.ts/platforms/panduck-unknown-wasm32/lib/.gitignore"];

for (const target of targets) {
    const file = path.join(root, target);
    if (fs.existsSync(file)) {
        fs.rmSync(file, { force: true });
        console.log(`removed ${target}`);
    }
}
