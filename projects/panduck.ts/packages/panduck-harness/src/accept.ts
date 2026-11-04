import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { runCase } from "./adapter.js";
import { evaluateExpectation } from "./assertions.js";
import { loadConversionCases, loadExpectation, expectationFile } from "./corpus.js";
import { resolveExpectation } from "@notedge/panduck-fixtures";
import type { Assertion, CaseDefinition, Expectation } from "./types.js";
import { writeJsonFile } from "./util.js";

const ASSERTION_KINDS = new Set([
    "exit-code",
    "report-status",
    "contains",
    "not-contains",
    "count",
    "loss-codes",
    "diagnostic-codes",
]);

function validateAssertion(value: unknown): Assertion {
    if (typeof value !== "object" || value === null) {
        throw new Error("added assertion must be a JSON object");
    }
    const record = value as Record<string, unknown>;
    if (typeof record.kind !== "string" || !ASSERTION_KINDS.has(record.kind)) {
        throw new Error(`unsupported assertion kind: ${String(record.kind)}`);
    }
    if ("leg" in record && typeof record.leg !== "number") {
        throw new Error("assertion `leg` must be a number");
    }
    if (record.kind === "contains" || record.kind === "not-contains" || record.kind === "count") {
        if (record.target !== "text" && record.target !== "html") {
            throw new Error(`assertion \`${record.kind}\` needs target "text" or "html"`);
        }
        if (typeof record.value !== "string" && typeof record.pattern !== "string") {
            throw new Error(`assertion \`${record.kind}\` needs \`value\` or \`pattern\``);
        }
    }
    return record as unknown as Assertion;
}

function findCase(caseId: string): CaseDefinition {
    const definition = loadConversionCases().find((entry) => entry.id === caseId);
    if (!definition) {
        throw new Error(`unknown conversion case: ${caseId}`);
    }
    return definition;
}

export type AcceptOptions = {
    readonly caseId: string;
    readonly assertionJson?: string;
    readonly assertionFile?: string;
    readonly author: string;
    readonly reason: string;
    readonly semantic: string;
};

/** Adds one assertion to an expectation and records who, why and which semantic moved. */
export function addAssertion(options: AcceptOptions): {
    readonly caseId: string;
    readonly assertionCount: number;
    readonly expectationPath: string;
} {
    if (options.assertionJson === undefined && options.assertionFile === undefined) {
        throw new Error("accept needs `--add` JSON or `--add-file`");
    }
    if (options.author.length === 0 || options.reason.length === 0 || options.semantic.length === 0) {
        throw new Error("accept needs `--by`, `--why` and `--semantic`");
    }
    const definition = findCase(options.caseId);
    const expectation = loadExpectation(definition.id, expectationFile(definition));
    const raw = options.assertionJson ?? readFileSync(resolve(options.assertionFile ?? ""), "utf8");
    const assertion = validateAssertion(JSON.parse(raw) as unknown);
    const serialized = JSON.stringify(assertion);
    if (expectation.assertions.some((entry) => JSON.stringify(entry) === serialized)) {
        throw new Error(`assertion already exists on \`${definition.id}\``);
    }

    const observation = runCase(definition);
    const probe: Expectation = { ...expectation, assertions: [assertion] };
    const probeResults = evaluateExpectation(observation, probe);
    const failed = probeResults.find((entry) => entry.status === "fail");
    if (failed) {
        throw new Error(`refusing to accept an assertion that does not pass: ${failed.detail}`);
    }

    const updated: Expectation = {
        ...expectation,
        assertions: [...expectation.assertions, assertion],
        provenance: [
            ...expectation.provenance,
            {
                author: options.author,
                reason: options.reason,
                semantic: options.semantic,
                updatedAt: new Date().toISOString(),
            },
        ],
    };
    const expectationPath = resolveExpectation(expectationFile(definition));
    writeJsonFile(expectationPath, updated);
    return {
        caseId: definition.id,
        assertionCount: updated.assertions.length,
        expectationPath,
    };
}
