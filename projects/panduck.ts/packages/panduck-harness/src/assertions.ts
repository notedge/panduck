import type {
    Assertion,
    AssertionResult,
    AssertionTarget,
    CaseObservation,
    Expectation,
    FailureCategory,
    LegObservation,
    LexedSnapshot,
    SnapshotCaseDefinition,
} from "./types.js";

function result(
    assertion: AssertionResult["assertion"],
    status: "pass" | "fail",
    detail: string,
    category?: FailureCategory,
): AssertionResult {
    return category && status === "fail" ? { assertion, status, detail, category } : { assertion, status, detail };
}

function legCategory(leg: LegObservation | undefined): FailureCategory {
    return leg?.error?.startsWith("missing-entry:") ? "undetermined" : "implementation-defect";
}

function legAt(observation: CaseObservation, legIndex: number): LegObservation | undefined {
    return observation.legs[legIndex];
}

function targetValue(leg: LegObservation, target: AssertionTarget): string | undefined {
    return target === "html" ? leg.html : leg.text;
}

function sorted(values: readonly string[]): string[] {
    return [...values].sort();
}

function compareStringSets(actual: readonly string[], expected: readonly string[], mode: "subset" | "exact"): boolean {
    const actualSorted = sorted(actual);
    const expectedSorted = sorted(expected);
    if (mode === "exact") {
        return actualSorted.length === expectedSorted.length && actualSorted.every((value, index) => value === expectedSorted[index]);
    }
    return expectedSorted.every((value) => actualSorted.includes(value));
}

function evaluateOne(observation: CaseObservation, assertion: Assertion): AssertionResult {
    const legIndex = "leg" in assertion ? (assertion.leg ?? 0) : 0;
    const leg = legAt(observation, legIndex);
    if (!leg) {
        return result(assertion, "fail", `observation has no leg ${legIndex}`, "implementation-defect");
    }
    if (leg.error) {
        const category = legCategory(leg);
        return result(assertion, "fail", `leg ${legIndex} failed before an observable output: ${leg.error}`, category);
    }
    switch (assertion.kind) {
        case "exit-code": {
            const ok = leg.exitCode === assertion.expected;
            return result(assertion, ok ? "pass" : "fail", `exit code ${leg.exitCode}, expected ${assertion.expected}`, ok ? undefined : legCategory(leg));
        }
        case "report-status": {
            const ok = leg.status === assertion.expected;
            return result(assertion, ok ? "pass" : "fail", `report status ${leg.status}, expected ${assertion.expected}`, ok ? undefined : legCategory(leg));
        }
        case "contains":
        case "not-contains": {
            const value = targetValue(leg, assertion.target);
            if (value === undefined) {
                return result(assertion, "fail", `leg ${legIndex} produced no ${assertion.target} output`, legCategory(leg));
            }
            const found = value.includes(assertion.value);
            const ok = assertion.kind === "contains" ? found : !found;
            return result(
                assertion,
                ok ? "pass" : "fail",
                assertion.kind === "contains"
                    ? `${assertion.target} ${found ? "contains" : "does not contain"} ${JSON.stringify(assertion.value)}`
                    : `${assertion.target} ${found ? "still contains" : "does not contain"} ${JSON.stringify(assertion.value)}`,
                ok ? undefined : "implementation-defect",
            );
        }
        case "count": {
            const value = targetValue(leg, assertion.target);
            if (value === undefined) {
                return result(assertion, "fail", `leg ${legIndex} produced no ${assertion.target} output`, legCategory(leg));
            }
            let pattern: RegExp;
            try {
                pattern = new RegExp(assertion.pattern, "g");
            } catch (error) {
                return result(assertion, "fail", `invalid count pattern ${JSON.stringify(assertion.pattern)}: ${String(error)}`, "stale-expectation");
            }
            const actual = value.match(pattern)?.length ?? 0;
            const ok = actual === assertion.expected;
            return result(
                assertion,
                ok ? "pass" : "fail",
                `${assertion.target} matched ${assertion.pattern} ${actual} times, expected ${assertion.expected}`,
                ok ? undefined : "implementation-defect",
            );
        }
        case "loss-codes": {
            const ok = compareStringSets(leg.lossCodes, assertion.expected, "exact");
            return result(
                assertion,
                ok ? "pass" : "fail",
                `loss codes ${JSON.stringify(sorted(leg.lossCodes))}, expected ${JSON.stringify(sorted(assertion.expected))}`,
                ok ? undefined : "implementation-defect",
            );
        }
        case "diagnostic-codes": {
            const ok = compareStringSets(leg.diagnosticCodes, assertion.expected, assertion.mode ?? "subset");
            return result(
                assertion,
                ok ? "pass" : "fail",
                `diagnostic codes ${JSON.stringify(sorted(leg.diagnosticCodes))}, expected ${assertion.mode ?? "subset"} of ${JSON.stringify(sorted(assertion.expected))}`,
                ok ? undefined : "implementation-defect",
            );
        }
        case "honest-success": {
            const value = targetValue(leg, assertion.target);
            if (value === undefined) {
                return result(assertion, "fail", `leg ${legIndex} produced no ${assertion.target} output`, legCategory(leg));
            }
            if (leg.status !== "success") {
                return result(
                    assertion,
                    "pass",
                    `report already admits a problem with status ${leg.status}, so honesty does not need the structural markers`,
                );
            }
            const missing = assertion.required.filter((marker) => !value.includes(marker));
            const ok = missing.length === 0;
            return result(
                assertion,
                ok ? "pass" : "fail",
                ok
                    ? `status success is backed by every required structural marker`
                    : `status success was reported while ${assertion.target} lost ${JSON.stringify(missing)}`,
                ok ? undefined : "implementation-defect",
            );
        }
    }
}

/** Evaluates every expectation assertion against a normalized observation. */
export function evaluateExpectation(
    observation: CaseObservation,
    expectation: Expectation,
): readonly AssertionResult[] {
    return expectation.assertions.map((assertion) => evaluateOne(observation, assertion));
}

function integrity(assertion: AssertionResult["assertion"], ok: boolean, detail: string): AssertionResult {
    return ok ? { assertion, status: "pass", detail } : { assertion, status: "fail", category: "undetermined", detail };
}

/** Evaluates the corpus and the unavailable conformance axes for a snapshot case. */
export function evaluateSnapshot(
    definition: SnapshotCaseDefinition,
    source: Uint8Array,
    snapshot: LexedSnapshot,
): readonly AssertionResult[] {
    const assertions: AssertionResult[] = [];
    assertions.push(
        integrity(
            { kind: "corpus-integrity", check: "snapshot-success" },
            snapshot.success === true,
            `snapshot success flag is ${snapshot.success}`,
        ),
    );

    const tokens = Array.isArray(snapshot.tokens) ? snapshot.tokens : [];
    const shapeOk =
        tokens.length > 0 &&
        snapshot.count === tokens.length &&
        tokens.every(
            (token) =>
                typeof token.kind === "string" &&
                typeof token.text === "string" &&
                typeof token.start === "number" &&
                typeof token.end === "number",
        );
    assertions.push(
        integrity(
            { kind: "corpus-integrity", check: "snapshot-shape" },
            shapeOk,
            `snapshot carries ${tokens.length} tokens but declares count ${snapshot.count}`,
        ),
    );

    const sourceBytes = Buffer.byteLength(source);
    let previousEnd = 0;
    let offsetsOk = true;
    for (const token of tokens) {
        if (token.start < previousEnd || token.end < token.start || token.end > sourceBytes) {
            offsetsOk = false;
            break;
        }
        previousEnd = token.end;
    }
    assertions.push(
        integrity(
            { kind: "corpus-integrity", check: "snapshot-offsets" },
            offsetsOk,
            `token offsets must be monotonic and inside 0..${sourceBytes}`,
        ),
    );

    assertions.push(
        integrity(
            { kind: "corpus-integrity", check: "snapshot-equality" },
            false,
            `no lexer token entry is exposed by the public Panduck binding, so \`${definition.id}\` cannot compare its snapshot`,
        ),
    );

    if (definition.oracle !== null) {
        assertions.push(
            integrity(
                { kind: "corpus-integrity", check: "oracle-summary" },
                false,
                `reference oracle ${JSON.stringify(definition.oracle)} has no exposed summary runner in this harness`,
            ),
        );
    }
    return assertions;
}

/** Aggregates assertion outcomes into the case category. */
export function aggregateCategory(results: readonly AssertionResult[]): FailureCategory | undefined {
    if (!results.some((entry) => entry.status === "fail")) {
        return undefined;
    }
    const failures = results.filter((entry) => entry.status === "fail");
    if (failures.some((entry) => entry.category === "implementation-defect")) {
        return "implementation-defect";
    }
    if (failures.some((entry) => entry.category === "stale-expectation")) {
        return "stale-expectation";
    }
    return "undetermined";
}
