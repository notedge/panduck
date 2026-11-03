import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildPanduckCli } from "./cli.js";
import { ExitCode } from "./exit-codes.js";

function packageVersion(): string {
    try {
        const root = join(dirname(fileURLToPath(import.meta.url)), "../../package.json");
        const pkg = JSON.parse(readFileSync(root, "utf8")) as { version?: string };
        return pkg.version ?? "0.0.0";
    } catch {
        return "0.0.0";
    }
}

export async function runCli(argv: string[]): Promise<number> {
    if (argv.length === 1 && (argv[0] === "--version" || argv[0] === "-V")) {
        console.log(packageVersion());
        return ExitCode.Success;
    }
    try {
        return await buildPanduckCli().parse(argv);
    } catch (error) {
        console.error(`error: ${error instanceof Error ? error.message : String(error)}`);
        return ExitCode.InvalidArgs;
    }
}
