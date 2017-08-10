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
};

const LOSS_VALUES = new Set<LossPolicy>(["allow", "warn", "deny"]);
const ASSET_VALUES = new Set<AssetPolicy>(["preserve", "copy", "embed", "drop"]);
const LINK_VALUES = new Set<LinkPolicy>(["preserve", "rewrite", "drop"]);
const DIAG_VALUES = new Set<DiagnosticsMode>(["silent", "human", "json"]);
const STAGE_VALUES = new Set<InspectStage>(["probe", "index", "decode", "validate"]);

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
    };
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

    constructor(field: string, value: string) {
        super(`invalid ${field}: ${value}`);
        this.field = field;
        this.value = value;
    }
}
