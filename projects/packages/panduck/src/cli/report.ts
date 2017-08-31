import type { CapabilityState } from "./format-registry.js";

export const REPORT_SCHEMA_VERSION = "panduck.report/v1";

export type ReportStatus =
    | "success"
    | "success_with_loss"
    | "partial"
    | "blocked"
    | "failed";

export type LossPolicy = "allow" | "warn" | "deny";

export type PanduckReport = {
    schema_version: string;
    tool: {
        name: string;
        version: string;
    };
    operation: string;
    status: ReportStatus;
    inputs: Array<{ path: string; format?: string }>;
    detection?: {
        outer?: string;
        inner?: string;
        format?: string;
        confidence?: "hint" | "verified" | "ambiguous";
        hints?: string[];
    };
    pipeline?: {
        reader?: string;
        writer?: string;
        stages?: string[];
        blocked_reason?: string;
    };
    parts?: string[];
    decoded_parts?: Array<{
        path: string;
        compression_method: number;
        compressed_size: number;
        uncompressed_size: number;
        decoded_size: number;
    }>;
    coverage?: {
        state: CapabilityState;
        read?: number;
        write?: number;
    };
    losses?: Array<{
        code: string;
        owner: string;
        severity: string;
        message: string;
        action?: string;
    }>;
    diagnostics?: Array<{
        owner: string;
        severity: string;
        message: string;
        code?: string;
    }>;
    outputs?: Array<{ path: string; published: boolean }>;
    budgets?: Record<string, number>;
    determinism?: {
        profile?: string;
        config?: string | null;
    };
    status_policy?: {
        loss: LossPolicy;
        strict: boolean;
        exit_code: number;
    };
};

export function createReport(input: {
    operation: string;
    toolVersion: string;
    status: ReportStatus;
    inputs: PanduckReport["inputs"];
    detection?: PanduckReport["detection"];
    pipeline?: PanduckReport["pipeline"];
    parts?: PanduckReport["parts"];
    decodedParts?: PanduckReport["decoded_parts"];
    coverage?: PanduckReport["coverage"];
    losses?: PanduckReport["losses"];
    diagnostics?: PanduckReport["diagnostics"];
    outputs?: PanduckReport["outputs"];
    budgets?: PanduckReport["budgets"];
    determinism?: PanduckReport["determinism"];
    statusPolicy?: PanduckReport["status_policy"];
}): PanduckReport {
    return {
        schema_version: REPORT_SCHEMA_VERSION,
        tool: { name: "panduck", version: input.toolVersion },
        operation: input.operation,
        status: input.status,
        inputs: input.inputs,
        detection: input.detection,
        pipeline: input.pipeline,
        parts: input.parts,
        decoded_parts: input.decodedParts,
        coverage: input.coverage,
        losses: input.losses ?? [],
        diagnostics: input.diagnostics ?? [],
        outputs: input.outputs ?? [],
        budgets: input.budgets ?? {},
        determinism: input.determinism,
        status_policy: input.statusPolicy,
    };
}
