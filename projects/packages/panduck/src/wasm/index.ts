import { loadPanduckWasm } from "./load.js";

export type { PanduckBindings, PanduckWasmOptions } from "../types.js";
export { loadPanduckWasm } from "./load.js";

let cached: Promise<import("../types.js").PanduckBindings> | undefined;

/** Cached WASM binding loader. */
export function loadPanduckWeb(): Promise<import("../types.js").PanduckBindings> {
    if (!cached) {
        cached = loadPanduckWasm();
    }
    return cached;
}
