import type { PanduckBindings, PanduckWasmOptions } from "../types.js";

type WasmModule = {
    default: (input?: { module_or_path?: string | URL }) => Promise<unknown>;
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
};

/** Load `@notedge/panduck-unknown-wasm32` and return the Panduck binding surface. */
export async function loadPanduckWasm(options: PanduckWasmOptions = {}): Promise<PanduckBindings> {
    const wasm = (await import("@notedge/panduck-unknown-wasm32/pkg/panduck_wasm.js")) as WasmModule;
    if (options.url) {
        await wasm.default({ module_or_path: options.url });
    } else {
        await wasm.default();
    }
    return {
        panduckVersion: () => wasm.panduckVersion(),
        supportedFormats: () => wasm.supportedFormats(),
        isSupportedFormat: (name) => wasm.isSupportedFormat(name),
    };
}
