export type ConvertResponse = {
    exitCode: number;
    text?: string;
    html?: string;
    binary?: Uint8Array;
    reportJson: string;
};

export type ConvertProjectResponse = {
    exitCode: number;
    indexMarkdown: string;
    publishedAssets: string[];
    publishedChapters: string[];
    unresolvedAssets: string[];
    outputDir: string;
    reportJson: string;
};

export type InspectIndexResponse = {
    exitCode: number;
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
    exitCode: number;
    format: string;
    outer: string;
    inner: string;
    parts: InspectDecodedPart[];
    reportJson: string;
};

/** One lexical token produced by a Panduck source grammar. */
export type LexToken = {
    /** Grammar token kind, such as `Heading1`, `TagName` or `Eof`. */
    kind: string;
    /** Source slice covered by this token. */
    text: string;
    /** Byte offset of the first source character. */
    start: number;
    /** Byte offset one past the last source character. */
    end: number;
    /** Whether the grammar classifies this token as trivia. */
    ignored: boolean;
};

export type LexResponse = {
    /** False when the grammar reported a fatal lexical error. */
    success: boolean;
    /** Tokens in source order, including grammar trivia. */
    tokens: LexToken[];
    /** Lexical diagnostics rendered as text. */
    diagnostics: string[];
};

/** Shared Panduck binding surface for Node-API and WebAssembly backends. */
export type PanduckBindings = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
    supportsConversion?: (from: string, to: string) => boolean;
    supportedConversions?: () => string[];
    convertDocument?: (
        from: string,
        to: string,
        inputPath: string,
        handlersJson?: string,
        budgetsJson?: string,
    ) => ConvertResponse;
    /** Convert in-memory bytes — the path Pandark Engine takes when the page is already fetched. */
    convertBytesMemory?: (
        from: string,
        to: string,
        label: string,
        bytes: Uint8Array,
        handlersJson?: string,
        budgetsJson?: string,
    ) => ConvertResponse;
    supportsMarkdownProject?: (from: string) => boolean;
    convertMarkdownProject?: (
        from: string,
        inputPath: string,
        outputDir: string,
        handlersJson?: string,
        budgetsJson?: string,
    ) => ConvertProjectResponse;
    /** Byte-input counterpart of `convertMarkdownProject` — executes `ConversionPlan.publish`. */
    convertMarkdownProjectBytes?: (
        from: string,
        label: string,
        bytes: Uint8Array,
        outputDir: string,
        handlersJson?: string,
        budgetsJson?: string,
    ) => ConvertProjectResponse;
    inspectIndex?: (inputPath: string) => InspectIndexResponse;
    inspectDecode?: (inputPath: string, partPath?: string) => InspectDecodeResponse;
    /** Lexically analyse source text with the grammar Panduck reads it as. */
    lexDocument?: (language: string, source: string) => LexResponse;
    installLoggerLogFile?: (path: string) => void;
    installConsoleLogFile?: (path: string) => void;
};

export type PanduckWasmOptions = {
    /** Browser URL for `panduck_wasm_bg.wasm` when not using the default package layout. */
    url?: string | URL;
};
