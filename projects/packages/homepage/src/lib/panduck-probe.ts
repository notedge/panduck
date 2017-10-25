import { loadPanduckWasm } from '@notedge/panduck/wasm';

export type PanduckProbe = {
    version: string;
    formats: string[];
    markdownOk: boolean;
    backend: 'node' | 'wasm';
};

/** Node probe for CI and local verify without a browser. */
export async function probePanduckNode(): Promise<PanduckProbe> {
    const { loadPanduckNode } = await import('@notedge/panduck/node');
    const binding = loadPanduckNode();
    const formats = binding.supportedFormats();
    return {
        version: binding.panduckVersion(),
        formats,
        markdownOk: binding.isSupportedFormat('markdown'),
        backend: 'node',
    };
}

/** Browser WASM probe via `@notedge/panduck/wasm`. */
export async function probePanduckWasm(): Promise<PanduckProbe> {
    const binding = await loadPanduckWasm();
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
