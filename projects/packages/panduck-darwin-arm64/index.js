import { createRequire } from "node:module";
import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const libDir = join(dirname(fileURLToPath(import.meta.url)), "lib");
const nodeName = readdirSync(libDir).find((name) => name.endsWith(".node"));
if (!nodeName) {
    throw new Error(`Missing .node binary in ${libDir}. Run npm run build:napi from the repo root.`);
}

const binding = require(join(libDir, nodeName));
export default binding;
