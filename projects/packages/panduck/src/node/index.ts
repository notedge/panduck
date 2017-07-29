import { loadPanduckNode } from "./load.js";

export type { PanduckBindings } from "../types.js";
export { loadPanduckNode } from "./load.js";

let cached: import("../types.js").PanduckBindings | undefined;

/** Cached Node-API binding loader. */
export function loadPanduckNative(): import("../types.js").PanduckBindings {
    if (!cached) {
        cached = loadPanduckNode();
    }
    return cached;
}
