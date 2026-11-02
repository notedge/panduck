export type PanduckProbe = {
    version: string;
    formats: string[];
    markdownOk: boolean;
    backend: 'node' | 'wasm' | 'browser-unavailable';
};

/**
 * Browser snapshot. The live WASM probe is disabled because the assembler packs
 * every statically visible import for the browser and cannot yet vendor the
 * multi-file `@notedge/panduck` package (`src/wasm/index.ts` pulls `./load.ts`).
 * Track the compiler fix, then restore `probePanduckWasm` / `probePanduckNode`.
 */
export function probePanduckBrowserUnavailable(): PanduckProbe {
    return {
        version: '(browser probe unavailable)',
        formats: ['markdown', 'docx', 'epub', 'html'],
        markdownOk: true,
        backend: 'browser-unavailable',
    };
}

/** Homepage runs in the browser, so the probe is always the unavailable snapshot. */
export async function probePanduck(): Promise<PanduckProbe> {
    return probePanduckBrowserUnavailable();
}
