export type { PanduckBindings, PanduckWasmOptions } from "./types.js";
export { loadPanduckNative, loadPanduckNode } from "./node/index.js";
export { loadPanduckWeb, loadPanduckWasm } from "./wasm/index.js";
export { buildPanduckCli, runCli, getFormatRegistry, ExitCode, REPORT_SCHEMA_VERSION } from "./cli/index.js";
export type { FormatRecord, PanduckReport, CapabilityState } from "./cli/index.js";

/** @deprecated Use `loadPanduckNode` or `loadPanduckNative`. */
export type PanduckNative = import("./types.js").PanduckBindings;
