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
    /** Wall-clock bound for the child process, in milliseconds. */
    timeoutMs?: number;
};

export function runPanduck(args: string[], options: PanduckCliOptions = {}): PanduckCliResult {
    const result = spawnSync(process.execPath, [panduckBin, ...args], {
        encoding: "utf8",
        input: options.stdin,
        timeout: options.timeoutMs,
    });
    return {
        code: result.status ?? 1,
        stdout: result.stdout,
        stderr: result.stderr,
    };
}
