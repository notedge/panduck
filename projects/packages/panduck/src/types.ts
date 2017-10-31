export type ConvertResponse = {
    exitCode: number;
    markdown?: string;
    html?: string;
    binary?: Uint8Array;
    reportJson: string;
};

export type ConvertProjectResponse = {
    exitCode: number;
    indexMarkdown: string;
    publishedAssets: string[];
    unresolvedAssets: string[];
    outputDir: string;
    reportJson: string;
};

export type InspectIndexResponse = {
    format: string;
    outer: string;
    inner: string;
    parts: string[];
    reportJson: string;
};

export type InspectDecodedPart = {
    path: string;
    compressionMethod: number;
    compressedSize: number;
    uncompressedSize: number;
    decodedSize: number;
};

export type InspectDecodeResponse = {
    format: string;
    outer: string;
    inner: string;
    parts: InspectDecodedPart[];
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
    supportsMarkdownProject?: (from: string) => boolean;
    convertMarkdownProject?: (from: string, inputPath: string, outputDir: string) => ConvertProjectResponse;
    inspectIndex?: (inputPath: string) => InspectIndexResponse;
    inspectDecode?: (inputPath: string, partPath?: string) => InspectDecodeResponse;
    installLoggerLogFile?: (path: string) => void;
    installConsoleLogFile?: (path: string) => void;
};

export type PanduckWasmOptions = {
    /** Browser URL for `panduck_wasm_bg.wasm` when not using the default package layout. */
    url?: string | URL;
};
