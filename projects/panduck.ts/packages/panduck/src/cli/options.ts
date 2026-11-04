import type { ParsedOptions } from "@vmz/commander";

import { ExitCode } from "./exit-codes.js";
import type { LossPolicy } from "./report.js";

export type AssetPolicy = "preserve" | "copy" | "embed" | "drop";
export type LinkPolicy = "preserve" | "rewrite" | "drop";
export type DiagnosticsMode = "silent" | "human" | "json";
export type InspectStage = "probe" | "index" | "decode" | "validate";

export type SharedCliOptions = {
    from?: string;
    to?: string;
    profile?: string;
    config?: string;
    json: boolean;
    budgets: Record<string, number>;
    loss: LossPolicy;
    strict: boolean;
    allowPartial: boolean;
    diagnostics: DiagnosticsMode;
    overwrite: boolean;
    logFile?: string;
};

const LOSS_VALUES = new Set<LossPolicy>(["allow", "warn", "deny"]);
const ASSET_VALUES = new Set<AssetPolicy>(["preserve", "copy", "embed", "drop"]);
const LINK_VALUES = new Set<LinkPolicy>(["preserve", "rewrite", "drop"]);
const DIAG_VALUES = new Set<DiagnosticsMode>(["silent", "human", "json"]);
const STAGE_VALUES = new Set<InspectStage>(["probe", "index", "decode", "validate"]);
/** The complete `--budget` key set for this build. */
export const BUDGET_KEYS = ["input_bytes", "output_bytes", "decompressed_bytes", "container_entries"] as const;

export function readFlag(options: ParsedOptions, key: string): boolean {
    return options[key] === true;
}

export function readString(options: ParsedOptions, key: string): string | undefined {
    const value = options[key];
    if (typeof value === "string" && value.length > 0) {
        return value;
    }
    return undefined;
}

export function readSharedOptions(options: ParsedOptions): SharedCliOptions {
    const loss = parseEnum(readString(options, "loss") ?? "warn", LOSS_VALUES, "loss");
    const diagnostics = parseEnum(
        readString(options, "diagnostics") ?? "human",
        DIAG_VALUES,
        "diagnostics",
    );
    return {
        from: readString(options, "from"),
        to: readString(options, "to"),
        profile: readString(options, "profile"),
        config: readString(options, "config"),
        json: readFlag(options, "json"),
        budgets: parseBudgets(options.budget),
        loss,
        strict: readFlag(options, "strict"),
        allowPartial: readFlag(options, "allow-partial"),
        diagnostics,
        overwrite: readFlag(options, "overwrite"),
        logFile: readString(options, "log-file"),
    };
}

const UNSUPPORTED_FLAGS = [
    "profile",
    "config",
    "allow-partial",
    "assets-dir",
    "asset-policy",
    "link-policy",
] as const;

/**
 * Each declared-but-inert flag belongs to a family with one replacement
 * story, so the rejection explains what the pipeline actually does instead of
 * only listing flag names.
 */
const UNSUPPORTED_FLAG_FAMILIES = [
    {
        keys: ["profile", "config"] as const,
        detail: "profile selection is not available in v0: conversions always use the built-in default profile",
    },
    {
        keys: ["assets-dir", "asset-policy", "link-policy"] as const,
        detail: "asset and link policy flags are not available in v0: assets stay inline and links stay verbatim",
    },
    {
        keys: ["allow-partial"] as const,
        detail: "partial-output allowance is not available in v0: every loss is reported as a warning or blocked by --strict",
    },
];

/**
 * Flags that are declared for discoverability but have no semantics in v0.
 * Accepting them silently would promise behavior the pipeline never applies,
 * so the commands reject them instead.
 */
export function unsupportedFlags(options: ParsedOptions): string[] {
    const flags: string[] = [];
    for (const key of UNSUPPORTED_FLAGS) {
        const value = options[key];
        if (value === undefined || value === false) {
            continue;
        }
        flags.push(`--${key}`);
    }
    return flags;
}

/**
 * Renders the rejection for inert flags. The first line keeps the flat flag
 * list, and each affected family adds one line explaining the v0 behavior.
 */
export function describeUnsupportedFlags(options: ParsedOptions): string[] {
    const flags = unsupportedFlags(options);
    if (flags.length === 0) {
        return [];
    }
    const lines = [`these flags are not supported in v0: ${flags.join(", ")}`];
    for (const family of UNSUPPORTED_FLAG_FAMILIES) {
        const given = family.keys.filter((key) => flags.includes(`--${key}`));
        if (given.length > 0) {
            lines.push(family.detail);
        }
    }
    return lines;
}

/** Budget keys the native container preflight can enforce today. */
export const CONTAINER_BUDGET_KEYS = ["decompressed_bytes", "container_entries"] as const;

/**
 * Serializes the container budgets the native layer enforces.
 * Returns `undefined` when the user did not set either key.
 */
export function containerBudgetsJson(budgets: Record<string, number>): string | undefined {
    const picked: Record<string, number> = {};
    for (const key of CONTAINER_BUDGET_KEYS) {
        const value = budgets[key];
        if (value !== undefined) {
            picked[key] = value;
        }
    }
    return Object.keys(picked).length > 0 ? JSON.stringify(picked) : undefined;
}

export function parseAssetPolicy(value: string | undefined): AssetPolicy {
    return parseEnum(value ?? "preserve", ASSET_VALUES, "asset-policy");
}

export function parseLinkPolicy(value: string | undefined): LinkPolicy {
    return parseEnum(value ?? "preserve", LINK_VALUES, "link-policy");
}

export function parseInspectStage(value: string | undefined): InspectStage {
    return parseEnum(value ?? "probe", STAGE_VALUES, "stage");
}

function parseEnum<T extends string>(value: string, allowed: Set<T>, name: string): T {
    if (!allowed.has(value as T)) {
        throw new CliOptionError(name, value);
    }
    return value as T;
}

export function parseBudgets(raw: string | boolean | string[] | undefined): Record<string, number> {
    const entries = normalizeRepeatable(raw);
    const budgets: Record<string, number> = {};
    for (const item of entries) {
        const eq = item.indexOf("=");
        if (eq <= 0) {
            throw new CliOptionError("budget", item);
        }
        const key = item.slice(0, eq).trim();
        if (!BUDGET_KEYS.includes(key as (typeof BUDGET_KEYS)[number])) {
            throw new CliOptionError("budget", item, `supported keys: ${BUDGET_KEYS.join(", ")}`);
        }
        const value = Number(item.slice(eq + 1).trim());
        if (!Number.isFinite(value) || value < 0) {
            throw new CliOptionError("budget", item);
        }
        budgets[key] = value;
    }
    return budgets;
}

function normalizeRepeatable(raw: string | boolean | string[] | undefined): string[] {
    if (raw === undefined || raw === false) {
        return [];
    }
    if (Array.isArray(raw)) {
        return raw;
    }
    if (typeof raw === "string") {
        return [raw];
    }
    return [];
}

export class CliOptionError extends Error {
    readonly field: string;
    readonly value: string;
    readonly exitCode = ExitCode.InvalidArgs;

    constructor(field: string, value: string, detail?: string) {
        super(`invalid ${field}: ${value}${detail ? ` (${detail})` : ""}`);
        this.field = field;
        this.value = value;
    }
}
