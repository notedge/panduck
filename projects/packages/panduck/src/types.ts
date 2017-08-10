export type ConvertResponse = {
    exitCode: number;
    markdown?: string;
    reportJson: string;
};

/** Shared Panduck binding surface for Node-API and WebAssembly backends. */
export type PanduckBindings = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
    supportsConversion?: (from: string, to: string) => boolean;
    supportedConversions?: () => string[];
    convertDocument?: (from: string, to: string, inputPath: string) => ConvertResponse;
};

export type PanduckWasmOptions = {
    /** Browser URL for `panduck_wasm_bg.wasm` when not using the default package layout. */
    url?: string | URL;
};
