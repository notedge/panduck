export type { PanduckBindings, PanduckWasmOptions } from "./types.js";
export { loadPanduckNative, loadPanduckNode } from "./node/index.js";
export { loadPanduckWeb, loadPanduckWasm } from "./wasm/index.js";
export { buildPanduckCli, runCli, getFormatRegistry, ExitCode, REPORT_SCHEMA_VERSION } from "./cli/index.js";
export type { FormatRecord, PanduckReport, CapabilityState } from "./cli/index.js";

// Public authoring surface — Converter + RendererPolicy.
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
} from "./converter.js";

// RendererPolicy — writer-level 策略层。
export {
    defineMarkdownPolicy,
    withHandlers,
} from "./renderer-policy.js";
export type {
    HandlerContext,
    PolicyHandlerResult,
    PolicyHandler,
    MarkdownPolicy,
    ConverterConfig,
} from "./renderer-policy.js";

// PassDescriptor — pass 的 I/O / capability 声明。
export type { Capability, PassDescriptor } from "./pass-descriptor.js";

// ConversionPlan — Panduck 产品级转换编排器（新合同，面向 notedown-ir）。
export { defineConversion } from "./conversion-plan.js";
export type {
    ConversionPlan,
    ConversionConfig,
    InputRoute,
    KnownInputFormat,
    KnownOutputFormat,
    OutputRoute,
    PublishStrategy,
    TransformPass,
} from "./conversion-plan.js";

/** @deprecated Use `loadPanduckNode` or `loadPanduckNative`. */
export type PanduckNative = import("./types.js").PanduckBindings;
