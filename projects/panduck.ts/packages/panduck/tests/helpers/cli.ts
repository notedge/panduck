import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const panduckBin = join(dirname(fileURLToPath(import.meta.url)), "../../bin/panduck");

export type PanduckCliResult = {
    code: number;
    stdout: string;
    stderr: string;
};

export type PanduckCliOptions = {
    /** Bytes piped to standard input, used by the `-` input path. */
    stdin?: string;
};

export function runPanduck(args: string[], options: PanduckCliOptions = {}): PanduckCliResult {
    const result = spawnSync(process.execPath, [panduckBin, ...args], {
        encoding: "utf8",
        input: options.stdin,
    });
    return {
        code: result.status ?? 1,
        stdout: result.stdout,
        stderr: result.stderr,
    };
}
