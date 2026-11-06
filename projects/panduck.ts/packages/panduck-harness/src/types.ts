/** Harness vocabulary shared by the runner, assertions and archive. */

export type { PdfLayoutRun } from "@notedge/panduck";

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
      }
    | {
          readonly kind: "honest-success";
          readonly leg?: number;
          readonly target: AssertionTarget;
          readonly required: readonly string[];
          /** Loss codes a `success_with_loss` report must actually name. */
          readonly losses?: readonly string[];
      }
    | {
          /**
           * Compares a layout run's reserved width with the advance painted into
           * the PDF, measured from glyph origins rather than glyph width sums.
           */
          readonly kind: "pdf-run-advance";
          readonly leg?: number;
          readonly runs: readonly {
              /** Exact `TextRun` text the layout pass reported. */
              readonly text: string;
              /** Allowed absolute difference in points. */
              readonly tolerance: number;
          }[];
      };

/** One layout run measured against the PDF glyph sequence. */
export type PdfRunAdvance = {
    readonly runText: string;
    readonly style: string;
    readonly fontFamily: string;
    readonly page: number;
    /** Origin of the run's first painted glyph, in PDF points. */
    readonly startX: number | null;
    /** Origin of the next painted glyph, which starts the follower run. */
    readonly nextX: number | null;
    /** Width the layout pass reserved for the run. */
    readonly reservedWidth: number;
    /** `nextX - startX`, or null when no follower glyph was painted. */
    readonly paintedAdvance: number | null;
    /** Absolute difference between the reserved width and the painted advance. */
    readonly delta: number | null;
    readonly note?: string;
};

/** Geometry derived from one PDF plus the layout runs that produced it. */
export type PdfGeometryObservation = {
    readonly pages: number;
    readonly glyphs: number;
    readonly runs: readonly PdfRunAdvance[];
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
    /** PDF glyph geometry, present when the leg produced a PDF and layout runs. */
    readonly geometry?: PdfGeometryObservation;
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
        readonly assertionDiff?: string;
        readonly observedStatus?: "pass" | "fail";
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
    /** Product grammar name accepted by `lexDocument`, or `null` when no reader exists. */
    readonly language: string | null;
    /** Set when the corpus grammar is explicitly outside Panduck's claimed scope. */
    readonly excluded?: string;
    readonly oracle: string | null;
    readonly comparison: string;
    /** Set when the recorded snapshot predates a known lexer fix in its source repo. */
    readonly snapshotLag?: {
        readonly reason: string;
        readonly evidence: string;
    };
};

/** Observation of one public `renderDiagram` call. */
export type DiagramObservation = {
    readonly success: boolean;
    readonly svg: string | null;
    readonly diagnostics: readonly {
        readonly code: string;
        readonly severity: string;
        readonly message: string;
        readonly spanText: string;
    }[];
    readonly error?: string;
};

/** Observation of one public runtime pass derivation. */
export type RuntimeObservation = {
    readonly passes: readonly string[];
    readonly scopes: readonly string[];
    readonly error?: string;
};

/** Observation of one public `renderChart` call. */
export type ChartObservation = {
    readonly success: boolean;
    readonly svg: string | null;
    readonly diagnostics: readonly {
        readonly code: string;
        readonly severity: string;
        readonly message: string;
        readonly spanText: string;
        readonly path: string;
    }[];
    readonly error?: string;
};

/** Observation of one public `validateDocumentGraph` call. */
export type IrObservation = {
    readonly valid: boolean;
    readonly issues: readonly {
        readonly code: string;
        readonly path: string;
        readonly detail: string;
    }[];
    readonly error?: string;
};

/** Observation of one public `lexDocument` call. */
export type LexObservation = {
    readonly success: boolean;
    readonly tokens: readonly {
        readonly kind: string;
        readonly text: string;
        readonly start: number;
        readonly end: number;
        readonly ignored: boolean;
    }[];
    readonly diagnostics: readonly string[];
    readonly error?: string;
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
