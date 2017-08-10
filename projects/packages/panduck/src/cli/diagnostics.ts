import type { DiagnosticsMode } from "./options.js";
import type { PanduckReport } from "./report.js";

export function emitHuman(message: string): void {
    console.error(message);
}

export function emitJson(value: unknown): void {
    console.log(JSON.stringify(value, null, 2));
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
        emitHuman(`[${item.owner}] ${item.severity}: ${item.message}`);
    }
}
