import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { resolveExpectation } from "@notedge/panduck-fixtures";

import { runCase } from "./adapter.js";
import { evaluateExpectation } from "./assertions.js";
import { expectationFile, loadConversionCases, loadExpectation, packageRoot } from "./corpus.js";
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
    "honest-success",
    "pdf-run-advance",
]);

function gitValue(arguments_: readonly string[]): string {
    return execFileSync("git", arguments_, { cwd: packageRoot, encoding: "utf8" }).trim();
}

/** `<git user.name> <git user.email> @ <HEAD sha>` so provenance cannot be typed by hand. */
export function resolveAuthor(): string {
    const name = gitValue(["config", "user.name"]);
    const email = gitValue(["config", "user.email"]);
    const head = gitValue(["rev-parse", "HEAD"]);
    if (name.length === 0 || email.length === 0) {
        throw new Error("accept needs `git config user.name` and `git config user.email`");
    }
    return `${name} <${email}> @ ${head}`;
}

export function validateAssertion(value: unknown): Assertion {
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
    if (
        record.kind === "contains" ||
        record.kind === "not-contains" ||
        record.kind === "count" ||
        record.kind === "honest-success"
    ) {
        if (record.target !== "text" && record.target !== "html") {
            throw new Error(`assertion \`${record.kind}\` needs target "text" or "html"`);
        }
    }
    if (record.kind === "contains" || record.kind === "not-contains") {
        if (typeof record.value !== "string") {
            throw new Error(`assertion \`${record.kind}\` needs \`value\``);
        }
    }
    if (record.kind === "count") {
        if (typeof record.pattern !== "string" || typeof record.expected !== "number") {
            throw new Error("assertion `count` needs `pattern` and `expected`");
        }
    }
    if (record.kind === "honest-success") {
        const required = record.required;
        if (!Array.isArray(required) || required.some((entry) => typeof entry !== "string") || required.length === 0) {
            throw new Error("assertion `honest-success` needs a non empty `required` string array");
        }
        const losses = record.losses;
        if (losses !== undefined && (!Array.isArray(losses) || losses.some((entry) => typeof entry !== "string"))) {
            throw new Error("assertion `honest-success` `losses` must be a string array");
        }
    }
    if (record.kind === "pdf-run-advance") {
        const runs = record.runs;
        if (
            !Array.isArray(runs) ||
            runs.length === 0 ||
            runs.some(
                (entry) =>
                    typeof entry !== "object" ||
                    entry === null ||
                    typeof (entry as Record<string, unknown>).text !== "string" ||
                    typeof (entry as Record<string, unknown>).tolerance !== "number",
            )
        ) {
            throw new Error('assertion `pdf-run-advance` needs a non empty `runs` array of `{ text, tolerance }`');
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

/** Identity used for duplicate detection and for replacing an assertion in place. */
function assertionIdentity(assertion: Assertion): string {
    switch (assertion.kind) {
        case "contains":
        case "not-contains":
            return `${assertion.kind}:${assertion.target ?? "text"}:${assertion.value}`;
        case "count":
            return `${assertion.kind}:${assertion.target ?? "text"}:${assertion.pattern}`;
        case "honest-success":
            return `${assertion.kind}:${assertion.target ?? "text"}`;
        default:
            return assertion.kind;
    }
}

export type AcceptOptions = {
    readonly caseId: string;
    readonly assertionJson?: string;
    readonly assertionFile?: string;
    readonly reason: string;
    readonly semantic: string;
    /** Replace the single assertion with the same identity instead of appending. */
    readonly replace?: boolean;
    /**
     * `oracle` writes a tightening that is expected to stay red until the owning
     * crate fixes the implementation. It still refuses duplicates and still
     * records the observed status, so it cannot be used to relax an expectation.
     */
    readonly redOracle?: boolean;
};

export type AcceptResult = {
    readonly caseId: string;
    readonly assertionCount: number;
    readonly expectationPath: string;
    readonly observedStatus: "pass" | "fail";
};

/** Adds one assertion to an expectation and records who, why, the diff and the observed status. */
export async function addAssertion(options: AcceptOptions): Promise<AcceptResult> {
    if (options.assertionJson === undefined && options.assertionFile === undefined) {
        throw new Error("accept needs `--add` JSON or `--add-file`");
    }
    if (options.reason.length === 0 || options.semantic.length === 0) {
        throw new Error("accept needs `--why` and `--semantic`");
    }
    const definition = findCase(options.caseId);
    const expectation = loadExpectation(definition.id, expectationFile(definition));
    const raw = options.assertionJson ?? readFileSync(resolve(options.assertionFile ?? ""), "utf8");
    const assertion = validateAssertion(JSON.parse(raw) as unknown);
    const serialized = JSON.stringify(assertion);
    const identity = assertionIdentity(assertion);
    const matches = expectation.assertions.filter((entry) => assertionIdentity(entry) === identity);
    if (options.replace === true) {
        if (matches.length !== 1) {
            throw new Error(
                `\`${definition.id}\` has ${matches.length} assertions with identity \`${identity}\`, so it cannot be replaced in place`,
            );
        }
    } else if (matches.length > 0) {
        throw new Error(`assertion already exists on \`${definition.id}\``);
    }

    const observation = await runCase(definition);
    const probe: Expectation = { ...expectation, assertions: [assertion] };
    const probeResults = evaluateExpectation(observation, probe);
    const failed = probeResults.find((entry) => entry.status === "fail");
    if (failed && options.redOracle !== true) {
        throw new Error(`refusing to accept an assertion that does not pass: ${failed.detail}`);
    }

    const assertions =
        options.replace === true
            ? expectation.assertions.map((entry) => (assertionIdentity(entry) === identity ? assertion : entry))
            : [...expectation.assertions, assertion];
    const updated: Expectation = {
        ...expectation,
        assertions,
        provenance: [
            ...expectation.provenance,
            {
                author: resolveAuthor(),
                reason: options.reason,
                semantic: options.semantic,
                updatedAt: new Date().toISOString(),
                assertionDiff: `${options.replace === true ? "~" : "+"} ${serialized}`,
                observedStatus: failed ? "fail" : "pass",
            },
        ],
    };
    const expectationPath = resolveExpectation(expectationFile(definition));
    writeJsonFile(expectationPath, updated);
    return {
        caseId: definition.id,
        assertionCount: updated.assertions.length,
        expectationPath,
        observedStatus: failed ? "fail" : "pass",
    };
}
