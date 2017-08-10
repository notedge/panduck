import { access, mkdtemp, rm } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import type { Cli, ParsedOptions } from "@vmz/commander";

import { loadBindings } from "../context.js";
import { emitHuman, emitJson } from "../diagnostics.js";
import { readSharedOptions } from "../options.js";

type DoctorCheck = {
    id: string;
    ok: boolean;
    message: string;
};

export function registerDoctorCommand(cli: Cli): void {
    cli.command("doctor", "cli.cmd.doctor")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runDoctor(options));
}

async function runDoctor(options: ParsedOptions): Promise<number> {
    const shared = readSharedOptions(options);
    const checks: DoctorCheck[] = [];

    checks.push({
        id: "node",
        ok: Number(process.versions.node.split(".")[0]) >= 20,
        message: `node ${process.version}`,
    });

    const bindings = loadBindings();
    if (bindings) {
        checks.push({
            id: "native",
            ok: true,
            message: `panduck-napi ${bindings.panduckVersion()}; formats=${bindings.supportedFormats().join(",")}`,
        });
    } else {
        checks.push({
            id: "native",
            ok: false,
            message: "native bindings are not installed for this platform",
        });
    }

    try {
        const dir = await mkdtemp(join(tmpdir(), "panduck-doctor-"));
        await access(dir, constants.W_OK);
        await rm(dir, { recursive: true, force: true });
        checks.push({ id: "tempdir", ok: true, message: tmpdir() });
    } catch (error) {
        checks.push({
            id: "tempdir",
            ok: false,
            message: error instanceof Error ? error.message : String(error),
        });
    }

    const ok = checks.every((check) => check.ok);
    const payload = {
        schema_version: "panduck.doctor/v1",
        status: ok ? "ok" : "issues",
        checks,
    };

    if (shared.json || options.json === true) {
        emitJson(payload);
    } else if (ok) {
        emitHuman("Panduck environment looks healthy");
        for (const check of checks) {
            emitHuman(`  ok  ${check.id}: ${check.message}`);
        }
    } else {
        emitHuman("Panduck environment has issues");
        for (const check of checks) {
            emitHuman(`  ${check.ok ? "ok" : "fail"}  ${check.id}: ${check.message}`);
        }
    }

    return ok ? 0 : 2;
}
