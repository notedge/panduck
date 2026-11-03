import { mkdir, rename, writeFile } from "node:fs/promises";
import { randomBytes } from "node:crypto";
import { basename, dirname, join } from "node:path";

import type { DiagnosticsMode } from "./options.js";
import type { PanduckReport } from "./report.js";

export function emitHuman(message: string): void {
    console.error(message);
}

export function emitJson(value: unknown): void {
    console.log(JSON.stringify(value, null, 2));
}

export async function writeReportFile(report: PanduckReport, path: string): Promise<void> {
    const dir = dirname(path);
    await mkdir(dir, { recursive: true });
    const temp = join(dir, `.${basename(path)}.${randomBytes(4).toString("hex")}.tmp`);
    await writeFile(temp, `${JSON.stringify(report, null, 2)}\n`, "utf8");
    await rename(temp, path);
}

export function emitReport(report: PanduckReport, mode: DiagnosticsMode, jsonRequested: boolean): void {
    if (jsonRequested || mode === "json") {
        emitJson(report);
        return;
    }
    if (mode === "silent") {
        return;
    }
    emitHuman(`${report.operation}: ${report.status}`);
    for (const item of report.diagnostics ?? []) {
        const { namespace, component } = item.origin;
        const text = item.message.fallback ?? item.message.key;
        emitHuman(`[${namespace}/${component}] ${item.severity}: ${text}`);
    }
}
