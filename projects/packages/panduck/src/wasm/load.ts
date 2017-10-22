import type { PanduckBindings, PanduckWasmOptions } from "../types.js";

const ARTIFACT_PACKAGE = "@notedge/panduck-unknown-wasm32";

type WasmBinding = {
    default: (input?: { module_or_path?: string | URL }) => Promise<unknown>;
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
};

/** Load the WASM artifact from `@notedge/panduck-unknown-wasm32`. */
export async function loadPanduckWasm(options: PanduckWasmOptions = {}): Promise<PanduckBindings> {
    const binding = (await import(ARTIFACT_PACKAGE)) as WasmBinding;
    if (options.url) {
        await binding.default({ module_or_path: options.url });
    } else {
        await binding.default();
    }
    return {
        panduckVersion: () => binding.panduckVersion(),
        supportedFormats: () => binding.supportedFormats(),
        isSupportedFormat: (name) => binding.isSupportedFormat(name),
    };
}
