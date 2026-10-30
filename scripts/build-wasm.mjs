import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const crateDir = join(root, "projects/panduck.rs/exports/panduck-wasm");
const outDir = join(root, "projects/panduck.ts/platforms/panduck-unknown-wasm32/lib");

execFileSync("wasm-pack", ["build", crateDir, "--target", "web", "--out-dir", outDir, "--release"], {
    cwd: root,
    stdio: "inherit",
});

execFileSync(process.execPath, [join(root, "scripts/ci/strip-wasm-gitignore.mjs")], {
    cwd: root,
    stdio: "inherit",
});
