// End-to-end floor check.
//
// `tsx --test` prints the aggregate as `ℹ pass N` (TAP output uses `# pass N`).
// The floor keeps the suite from shrinking silently: deleting cases turns the
// CI job red instead of lowering the bar.

import { readFileSync } from "node:fs";

const [, , logPath, floorArg] = process.argv;
const floor = Number.parseInt(floorArg ?? "", 10);

if (!logPath || !Number.isInteger(floor) || floor <= 0) {
    console.error("usage: node scripts/ci/check-e2e-floor.mjs <log> <floor>");
    process.exit(2);
}

const text = readFileSync(logPath, "utf8");
const passes = [...text.matchAll(/^\s*[\u2139#]\s*pass\s+(\d+)\s*$/gm)].map((match) => Number.parseInt(match[1], 10));
const failures = [...text.matchAll(/^\s*[\u2139#]\s*fail\s+(\d+)\s*$/gm)].map((match) => Number.parseInt(match[1], 10));

if (passes.length === 0) {
    console.error(`e2e floor: no node:test summary in ${logPath}`);
    process.exit(1);
}

const failed = failures.at(-1) ?? 0;
if (failed > 0) {
    console.error(`e2e floor: ${failed} failing test(s)`);
    process.exit(1);
}

const passed = passes.at(-1);
if (passed < floor) {
    console.error(`e2e floor: ${passed} passed, below the pinned floor of ${floor}`);
    process.exit(1);
}

console.log(`e2e floor: ${passed} passed, floor ${floor}`);
