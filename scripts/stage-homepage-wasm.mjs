/**
 * Stage Panduck WASM for homepage `public/panduck_wasm_bg.wasm`.
 *
 * Local: run `pnpm run build:wasm` when pkg is missing.
 * CI skip: copy from installed `@notedge/panduck-unknown-wasm32` when present.
 */

import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const wasmPkg = path.join(root, 'projects/packages/panduck-unknown-wasm32/pkg/panduck_wasm_bg.wasm');
const publicDir = path.join(root, 'projects/packages/homepage/public');
const publicWasm = path.join(publicDir, 'panduck_wasm_bg.wasm');
const skipRust = process.env.PANDUCK_HOMEPAGE_SKIP_WASM === '1' || process.env.PANDUCK_HOMEPAGE_SKIP_WASM === 'true';

function publishedWasmCandidates() {
    return [
        path.join(root, 'projects/packages/homepage/node_modules/@notedge/panduck-unknown-wasm32/pkg/panduck_wasm_bg.wasm'),
        path.join(root, 'projects/packages/panduck-unknown-wasm32/pkg/panduck_wasm_bg.wasm'),
        path.join(root, 'node_modules/@notedge/panduck-unknown-wasm32/pkg/panduck_wasm_bg.wasm'),
    ];
}

function stageWasm(from) {
    mkdirSync(publicDir, { recursive: true });
    copyFileSync(from, publicWasm);
    console.log(`stage-homepage-wasm: ${publicWasm} ← ${from}`);
}

if (skipRust) {
    const fromPub = publishedWasmCandidates().find((p) => existsSync(p));
    if (fromPub) {
        stageWasm(fromPub);
        process.exit(0);
    }
    if (existsSync(publicWasm)) {
        console.log(`stage-homepage-wasm: keep existing ${publicWasm}`);
        process.exit(0);
    }
    console.warn('stage-homepage-wasm: skip rust and no wasm in node_modules');
    process.exit(0);
}

if (!existsSync(wasmPkg)) {
    console.log('stage-homepage-wasm: building wasm via pnpm run build:wasm');
    const r = spawnSync('pnpm', ['run', 'build:wasm'], { cwd: root, stdio: 'inherit', shell: true });
    if (r.status !== 0) process.exit(r.status ?? 1);
}

if (!existsSync(wasmPkg)) {
    console.error(`stage-homepage-wasm: missing ${wasmPkg}`);
    process.exit(1);
}

stageWasm(wasmPkg);
