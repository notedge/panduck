import assert from "node:assert/strict";
import test from "node:test";

import { parseReportEnvelope } from "../src/diagnostic.js";

function report(diagnostics: unknown[], losses: unknown[] = []): string {
    return JSON.stringify({
        schema_version: "panduck.report/v0",
        status: "success",
        diagnostics: { schemaVersion: 1, diagnostics, truncated: false, dropped: 0 },
        losses,
    });
}

const boostDiagnostic = {
    code: "panduck.semantic.loss",
    severity: "warning",
    origin: { namespace: "panduck", component: "convert", stage: "loss" },
    message: {
        key: "panduck.semantic.loss",
        args: [["count", { u64: 1 }]],
        fallback: "one structure was dropped",
    },
    primary: null,
    secondary: [],
    notes: [],
    helps: [],
    cause: null,
    actions: [],
};

test("reads the boost diagnostic shape with nested message and origin", () => {
    const parsed = parseReportEnvelope(report([boostDiagnostic]), { component: "convert" });

    assert.equal(parsed.diagnostics.length, 1);
    const diagnostic = parsed.diagnostics[0];
    assert.equal(diagnostic.code, "panduck.semantic.loss");
    assert.equal(diagnostic.severity, "warning");
    assert.deepEqual(diagnostic.origin, {
        namespace: "panduck",
        component: "convert",
        stage: "loss",
    });
    assert.deepEqual(diagnostic.message, {
        key: "panduck.semantic.loss",
        args: [["count", { u64: 1 }]],
        fallback: "one structure was dropped",
    });
});

test("rejects flattened legacy diagnostics instead of accepting a second shape", () => {
    const parsed = parseReportEnvelope(
        report([
            {
                code: "panduck.convert.rust-loss",
                severity: "warning",
                message_key: "panduck.semantic.loss",
                fallback: "legacy flat diagnostic",
                args: { count: 1 },
            },
        ]),
        { component: "convert" },
    );

    assert.equal(parsed.diagnostics.length, 1);
    assert.equal(parsed.diagnostics[0].code, "panduck.convert.report-corrupt");
    assert.equal(parsed.diagnostics[0].severity, "error");
});

test("rejects severities outside the boost enum", () => {
    const parsed = parseReportEnvelope(
        report([{ ...boostDiagnostic, severity: "warn" }]),
        { component: "convert" },
    );

    assert.equal(parsed.diagnostics[0].code, "panduck.convert.report-corrupt");
});

test("loss count comes only from IR loss markers, never from diagnostic codes", () => {
    const parsed = parseReportEnvelope(
        report([{ ...boostDiagnostic, code: "panduck.convert.structure-loss", severity: "warning" }]),
        { component: "convert" },
    );

    assert.equal(parsed.diagnostics[0].code, "panduck.convert.structure-loss");
    assert.equal(parsed.losses.length, 0);
});

test("reads notedown loss markers as real statistics", () => {
    const parsed = parseReportEnvelope(
        report([], [{ code: "panduck.semantic.loss", message: "dropped", status: "Partial" }]),
        { component: "convert" },
    );

    assert.deepEqual(parsed.losses, [
        { code: "panduck.semantic.loss", message: "dropped", status: "Partial" },
    ]);
});
