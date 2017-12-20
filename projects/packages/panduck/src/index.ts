export type { PanduckBindings, PanduckWasmOptions } from "./types.js";
export { loadPanduckNative, loadPanduckNode } from "./node/index.js";
export { loadPanduckWeb, loadPanduckWasm } from "./wasm/index.js";
export { buildPanduckCli, runCli, getFormatRegistry, ExitCode, REPORT_SCHEMA_VERSION } from "./cli/index.js";
export type { FormatRecord, PanduckReport, CapabilityState } from "./cli/index.js";

// New authoring surface.
export { Converter } from "./converter.js";
export type {
    ConverterInput,
    ConverterPageLike,
    ConverterArtifactLike,
    ConverterResult,
    ConverterStatus,
    DiagnosticRecord,
    DiagnosticSeverity,
    ToMarkdownOptions,
    LossPolicy,
} from "./converter.js";

// Rule-plan builder.
export {
    RuleBuilder,
    InputScope,
    TransformScope,
    OutputScope,
} from "./rule-builder.js";
export type {
    RulePlan,
    RuleStep,
    RuleOp,
    RuleDomain,
    InputOp,
    TransformOp,
    OutputOp,
} from "./rule-builder.js";

/** @deprecated Use `loadPanduckNode` or `loadPanduckNative`. */
export type PanduckNative = import("./types.js").PanduckBindings;
