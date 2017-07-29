import { loadPanduckNode } from '@notedge/panduck/node';
import { loadPanduckWasm } from '@notedge/panduck/wasm';

export type PanduckProbe = {
    version: string;
    formats: string[];
    markdownOk: boolean;
    backend: 'node' | 'wasm';
};

/** Node probe for CI and local verify without a browser. */
export function probePanduckNode(): PanduckProbe {
    const binding = loadPanduckNode();
    const formats = binding.supportedFormats();
    return {
        version: binding.panduckVersion(),
        formats,
        markdownOk: binding.isSupportedFormat('markdown'),
        backend: 'node',
    };
}

function browserWasmUrl(): URL | undefined {
    if (typeof location === 'undefined') return undefined;
    return new URL('/panduck_wasm_bg.wasm', location.origin);
}

/** Browser WASM probe with optional staged public wasm URL. */
export async function probePanduckWasm(): Promise<PanduckProbe> {
    const url = browserWasmUrl();
    const binding = await loadPanduckWasm(url ? { url } : {});
    const formats = binding.supportedFormats();
    return {
        version: binding.panduckVersion(),
        formats,
        markdownOk: binding.isSupportedFormat('markdown'),
        backend: 'wasm',
    };
}

/** Prefer WASM in browsers, Node bindings elsewhere. */
export async function probePanduck(): Promise<PanduckProbe> {
    if (typeof window !== 'undefined') {
        return probePanduckWasm();
    }
    return probePanduckNode();
}
