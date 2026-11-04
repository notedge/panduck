/** Harness vocabulary shared by the runner, assertions and archive. */

export type CaseKind = "convert" | "roundtrip" | "rejection" | "lexer-snapshot";

export type AssertionStatus = "pass" | "fail";

export type FailureCategory = "implementation-defect" | "stale-expectation" | "undetermined";

export type AssertionTarget = "text" | "html";

export type Assertion =
    | { readonly kind: "exit-code"; readonly leg?: number; readonly expected: number }
    | {
          readonly kind: "report-status";
          readonly leg?: number;
          readonly expected: "success" | "success_with_loss" | "failed";
      }
    | {
          readonly kind: "contains";
          readonly leg?: number;
          readonly target: AssertionTarget;
          readonly value: string;
      }
    | {
          readonly kind: "not-contains";
          readonly leg?: number;
          readonly target: AssertionTarget;
          readonly value: string;
      }
    | {
          readonly kind: "count";
          readonly leg?: number;
          readonly target: AssertionTarget;
          readonly pattern: string;
          readonly expected: number;
      }
    | { readonly kind: "loss-codes"; readonly leg?: number; readonly expected: readonly string[] }
    | {
          readonly kind: "diagnostic-codes";
          readonly leg?: number;
          readonly expected: readonly string[];
          readonly mode?: "subset" | "exact";
      };

/** One conversion leg observed from the public Panduck binding. */
export type LegObservation = {
    readonly from: string;
    readonly to: string;
    readonly label: string;
    readonly exitCode: number;
    readonly status: string;
    readonly lossCodes: readonly string[];
    readonly diagnosticCodes: readonly string[];
    readonly text?: string;
    readonly html?: string;
    readonly binaryBytes?: number;
    readonly binarySha256?: string;
    readonly reportSha256?: string;
    readonly error?: string;
};

export type CapabilitySnapshot = {
    readonly version: string;
    readonly formats: readonly string[];
    readonly conversions: readonly string[];
};

/** Normalized observation for one harness case. */
export type CaseObservation = {
    readonly caseId: string;
    readonly kind: CaseKind;
    readonly legs: readonly LegObservation[];
    readonly durationMs: number;
    readonly capabilities: CapabilitySnapshot;
    readonly details?: Readonly<Record<string, unknown>>;
};

export type AssertionResult = {
    readonly assertion: Assertion | { readonly kind: "corpus-integrity"; readonly check: string };
    readonly status: AssertionStatus;
    readonly category?: FailureCategory;
    readonly detail: string;
};

export type CaseResult = {
    readonly id: string;
    readonly title: string;
    readonly kind: CaseKind;
    readonly status: "pass" | "fail";
    readonly category?: FailureCategory;
    readonly assertions: readonly AssertionResult[];
    readonly durationMs: number;
    readonly oracle?: string | null;
    readonly gap?: string;
};

export type GapRecord = {
    readonly id: string;
    readonly repo: string;
    readonly capability: string;
    readonly neededEntry: string;
    readonly evidence: string;
    readonly affectedCases: readonly string[];
};

export type RunReport = {
    readonly schema: "panduck.harness.run";
    readonly runId: string;
    readonly startedAt: string;
    readonly durationMs: number;
    readonly suite: string;
    readonly environment: {
        readonly node: string;
        readonly platform: string;
        readonly arch: string;
        readonly panduckVersion: string;
        readonly formats: readonly string[];
        readonly conversions: readonly string[];
        readonly bindingPath?: string;
        readonly bindingInsideRepo?: boolean;
    };
    readonly totals: {
        readonly cases: number;
        readonly passed: number;
        readonly failed: number;
        readonly assertions: number;
        readonly assertionsPassed: number;
        readonly assertionsFailed: number;
    };
    readonly classifications: Readonly<Record<FailureCategory, number>>;
    readonly cases: readonly CaseResult[];
    readonly gaps: readonly GapRecord[];
};

export type Expectation = {
    readonly schema: "panduck.harness.expectation";
    readonly caseId: string;
    readonly assertions: readonly Assertion[];
    readonly provenance: readonly {
        readonly author: string;
        readonly reason: string;
        readonly semantic: string;
        readonly updatedAt: string;
    }[];
};

/** Definition of a conversion, roundtrip or rejection case. */
export type CaseDefinition = {
    readonly id: string;
    readonly kind: "convert" | "roundtrip" | "rejection";
    readonly title: string;
    readonly input: string;
    readonly from: string;
    readonly to?: string;
    readonly via?: string;
    readonly expectation: string;
};

/** Definition of a manifest driven lexer snapshot case. */
export type SnapshotCaseDefinition = {
    readonly id: string;
    readonly kind: "lexer-snapshot";
    readonly title: string;
    readonly input: string;
    readonly snapshot: string;
    readonly oracle: string | null;
    readonly comparison: string;
};

export type LexedSnapshot = {
    readonly success: boolean;
    readonly count: number;
    readonly tokens: readonly {
        readonly kind: string;
        readonly text: string;
        readonly start: number;
        readonly end: number;
    }[];
};

export type ConversionCatalog = {
    readonly schema: "panduck.harness.cases";
    readonly cases: readonly CaseDefinition[];
};
