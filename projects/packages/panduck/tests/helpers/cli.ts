import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const panduckBin = join(dirname(fileURLToPath(import.meta.url)), "../../bin/panduck.mjs");

export type PanduckCliResult = {
    code: number;
    stdout: string;
    stderr: string;
};

export function runPanduck(args: string[]): PanduckCliResult {
    const result = spawnSync(process.execPath, [panduckBin, ...args], { encoding: "utf8" });
    return {
        code: result.status ?? 1,
        stdout: result.stdout,
        stderr: result.stderr,
    };
}
