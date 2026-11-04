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

/** One diagram diagnostic, fatal or non-fatal. */
export type DiagramDiagnostic = {
    /** Stable diagnostic code such as `davinci.diagram.unsupported`. */
    code: string;
    /** `error`, `warning` or `info`. */
    severity: string;
    /** Human readable message. */
    message: string;
    /** Source slice the diagnostic points at, empty when it has no span. */
    spanText: string;
};

export type DiagramRenderResponse = {
    /** False when the source was rejected or rendering failed. */
    success: boolean;
    /** SVG document for accepted sources. */
    svg: string | null;
    /** Diagnostics in source order. */
    diagnostics: DiagramDiagnostic[];
};

/** One structural invariant violation found on a document graph. */
export type IrValidationIssue = {
    /** Invariant name such as `UnknownInlineLinkTarget` or `ContainmentCycle`. */
    code: string;
    /** Graph location the issue points at, such as `node:7` or `relation:3`. */
    path: string;
    /** Human readable explanation of the violation. */
    detail: string;
};

export type IrValidationResponse = {
    /** True when the graph satisfies every structural invariant. */
    valid: boolean;
    /** Violations in the order the validator discovered them. */
    issues: IrValidationIssue[];
    /** Set when the payload was not a deserializable document graph. */
    error: string | null;
};

/** One diagnostic produced while lowering or rendering a chart. */
export type ChartDiagnostic = {
    /** Stable diagnostic code such as `plotter.chart.unsupported_mark`. */
    code: string;
    /** Severity label such as `error` or `warning`. */
    severity: string;
    /** Human readable explanation. */
    message: string;
    /** Source text covered by the diagnostic, empty when it has no span. */
    spanText: string;
    /** JSON path inside the Vega-Lite spec, empty when it has none. */
    path: string;
};

export type ChartRenderResponse = {
    /** True when the spec lowered and rendered without an error diagnostic. */
    success: boolean;
    /** Rendered SVG document, `null` on failure. */
    svg: string | null;
    /** Diagnostics emitted while lowering and rendering. */
    diagnostics: ChartDiagnostic[];
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
    /** Render a diagram source through the matching plotter frontend. */
    renderDiagram?: (language: string, source: string) => DiagramRenderResponse;
    /** Validate a serialized `document.ir` graph against its structural invariants. */
    validateDocumentGraph?: (graphJson: string) => IrValidationResponse;
    /** Render a Vega-Lite spec to SVG through `plotter.chart`. */
    renderChart?: (vegaLiteJson: string) => ChartRenderResponse;
    installLoggerLogFile?: (path: string) => void;
    installConsoleLogFile?: (path: string) => void;
};

export type PanduckWasmOptions = {
    /** Browser URL for `panduck_wasm_bg.wasm` when not using the default package layout. */
    url?: string | URL;
};
