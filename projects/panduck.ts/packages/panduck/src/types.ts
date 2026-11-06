/** Probe response for a container index pass. */
export type ProbeIndexResponse = {
    /** True when the container was indexed. */
    success: boolean;
    /** Detected container format. */
    format: string;
    /** Outer container label from Acorn detection. */
    outer: string;
    /** Inner payload label from Acorn detection. */
    inner: string;
    /** OPC member paths discovered in the package. */
    parts: string[];
    /** Serialized diagnostic envelope. */
    diagnosticsJson: string;
};

export type ProbeDecodedPart = {
    /** OPC member path inside the package. */
    path: string;
    /** ZIP compression method identifier. */
    compressionMethod: number;
    /** Compressed byte length stored in the archive. */
    compressedSize: number;
    /** Uncompressed byte length declared in the archive. */
    uncompressedSize: number;
    /** Decoded payload byte length after inflation. */
    decodedSize: number;
};

/** Probe response for a container decode pass. */
export type ProbeDecodeResponse = {
    /** True when the container was decoded. */
    success: boolean;
    /** Detected container format. */
    format: string;
    /** Outer container label from Acorn detection. */
    outer: string;
    /** Inner payload label from Acorn detection. */
    inner: string;
    /** Per-member decode summaries for the package. */
    parts: ProbeDecodedPart[];
    /** Serialized diagnostic envelope. */
    diagnosticsJson: string;
};

/** Result of reading bytes into a serialized `document.ir` graph. */
export type ImportResponse = {
    /** True when the bytes were read into IR. */
    success: boolean;
    /** Serialized graph, absent on failure. */
    graphJson: string | null;
    /** Serialized coverage records for the imported graph. */
    coverageJson: string;
    /** Serialized loss records with source spans. */
    lossesJson: string;
    /** Serialized diagnostic envelope for the import stage. */
    diagnosticsJson: string;
};

/** Result of running transform passes over serialized IR. */
export type TransformResponse = {
    /** True when the graph was parsed and transformed. */
    success: boolean;
    /** Serialized graph after the passes ran. */
    graphJson: string;
    /** Serialized transform pass reports for the plan descriptor. */
    passesJson: string;
    /** Serialized diagnostic envelope for the transform stage. */
    diagnosticsJson: string;
};

/** Result of writing a graph to one target format. */
export type ExportResponse = {
    /** True when the graph was written. */
    success: boolean;
    /** Text output for text targets such as `markdown`. */
    text?: string | null;
    /** HTML output for the HTML target. */
    html?: string | null;
    /** Binary output for container targets such as `docx`. */
    binary?: Uint8Array | null;
    /** Serialized coverage records after the export stage. */
    coverageJson: string;
    /** Serialized loss records after the export stage. */
    lossesJson: string;
    /** Serialized diagnostic envelope for the export stage. */
    diagnosticsJson: string;
};

/** One Markdown project chapter. */
export type ProjectChapter = {
    /** Project-relative chapter path. */
    path: string;
    /** Markdown body for this chapter. */
    markdown: string;
};

/** One materialized Markdown project asset. */
export type ProjectAsset = {
    /** Project-relative asset path. */
    path: string;
    /** Raw asset bytes. */
    bytes: Uint8Array;
};

/** Result of exporting a graph as a Markdown project payload. */
export type ProjectResponse = {
    /** True when the graph was exported as a Markdown project. */
    success: boolean;
    /** Primary Markdown body for `index.md`. */
    indexMarkdown: string;
    /** Additional chapter files. */
    chapters: ProjectChapter[];
    /** Materialized asset files. */
    assets: ProjectAsset[];
    /** Image sources that could not be materialized. */
    unresolvedAssets: string[];
    /** Serialized coverage records after the export stage. */
    coverageJson: string;
    /** Serialized loss records after the export stage. */
    lossesJson: string;
    /** Serialized diagnostic envelope for the export stage. */
    diagnosticsJson: string;
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
    /** Stable diagnostic code such as `davinci.chart.unsupported-mark`. */
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

/** Ordered incremental passes implied by one runtime action or scope set. */
export type RuntimePassResponse = {
    /** Passes in execution order, from upstream data to downstream paint. */
    passes: string[];
    /** Invalidation scopes that produced the passes. */
    scopes: string[];
    /** Set when the requested action or scope is outside the runtime vocabulary. */
    error: string | null;
};

/** One entry from an EPUB navigation table of contents. */
export type ProbeNavigationEntry = {
    /** Visible navigation label. */
    label: string;
    /** Manifest-relative href target. */
    href: string;
    /** Nested navigation entries. */
    children: ProbeNavigationEntry[];
};

/** Probe response for an EPUB navigation document. */
export type ProbeNavigationResponse = {
    /** True when the document yielded at least one entry. */
    success: boolean;
    /** Entries in document order. */
    entries: ProbeNavigationEntry[];
    /** Serialized diagnostic envelope. */
    diagnosticsJson: string;
};

/** Result of reading an Office package through its native model into IR. */
export type OfficeImportResponse = {
    /** True when the package was read and projected into IR. */
    success: boolean;
    /** Native model kind, `workbook` or `presentation`. */
    kind: string;
    /** Sheet names in workbook order, empty for presentations. */
    sheets: string[];
    /** Slide count, zero for workbooks. */
    slides: number;
    /** Serialized graph, absent on failure. */
    graphJson: string | null;
    /** Serialized coverage records for the imported graph. */
    coverageJson: string;
    /** Serialized loss records with source spans. */
    lossesJson: string;
    /** Serialized diagnostic envelope for the import stage. */
    diagnosticsJson: string;
};

/** Shared Panduck binding surface for Node-API and WebAssembly backends. */
/** One text run whose horizontal width `document-layout` reserved before painting. */
export type PdfLayoutRun = {
    /** Zero based page index the run was placed on. */
    page: number;
    /** Left edge in points, measured from the page left. */
    x: number;
    /** Baseline in points, measured from the page top. */
    y: number;
    /** Reserved width in points, measured with the same advances the line breaker used. */
    width: number;
    /** Font size in points. */
    size: number;
    /** Text carried by the run. */
    text: string;
    /** Resolved font family name. */
    fontFamily: string;
    /** Resolved face style name such as `Regular` or `Italic`. */
    style: string;
};

/** Page box produced by the layout pass. */
export type PdfLayoutPage = {
    /** Page width in points. */
    width: number;
    /** Page height in points. */
    height: number;
};

/** Layout runs paired with the PDF bytes painted from the same graph. */
export type PdfLayoutProbeResponse = {
    /** True when the probe produced a PDF. */
    success: boolean;
    /** PDF bytes rendered by the production writer. */
    pdf?: Uint8Array;
    /** Text runs in paint order. */
    runs: PdfLayoutRun[];
    /** Page boxes in document order. */
    pages: PdfLayoutPage[];
    /** Layout diagnostics rendered as text. */
    diagnostics: string[];
    /** Failure detail when no PDF was produced. */
    error: string | null;
};

export type PanduckBindings = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
    supportsConversion?: (from: string, to: string) => boolean;
    supportedConversions?: () => string[];
    /** Index a DOCX package from in-memory bytes. */
    probeContainerIndex?: (label: string, bytes: Uint8Array) => ProbeIndexResponse;
    /** Decode DOCX package members from in-memory bytes. */
    probeContainerDecode?: (label: string, bytes: Uint8Array, partPath?: string) => ProbeDecodeResponse;
    /** Parse an EPUB navigation document into its table of contents. */
    probeNavigationToc?: (label: string, bytes: Uint8Array) => ProbeNavigationResponse;
    /** Read bytes into serialized IR. `assets` carries caller-owned HTML image bytes by authored target. */
    importDocument?: (
        from: string,
        label: string,
        bytes: Uint8Array,
        budgetsJson?: string,
        assets?: Record<string, Uint8Array>,
    ) => ImportResponse;
    /** Read a PPTX or XLSX package through its native model into serialized IR. */
    importOfficeDocument?: (from: string, label: string, bytes: Uint8Array) => OfficeImportResponse;
    /** Run transform passes over serialized IR. */
    transformDocument?: (graphJson: string, transformsJson?: string) => TransformResponse;
    /** Write serialized IR to one target format. */
    exportDocument?: (to: string, graphJson: string) => ExportResponse;
    supportsMarkdownProject?: (from: string) => boolean;
    /** Export serialized IR as an in-memory Markdown project payload. */
    exportMarkdownProject?: (graphJson: string) => ProjectResponse;
    /** Lexically analyse source text with the grammar Panduck reads it as. */
    lexDocument?: (language: string, source: string) => LexResponse;
    /** Render a diagram source through the matching plotter frontend. */
    renderDiagram?: (language: string, source: string) => DiagramRenderResponse;
    /** Validate a serialized `document.ir` graph against its structural invariants. */
    validateDocumentGraph?: (graphJson: string) => IrValidationResponse;
    /** Render a Vega-Lite spec to SVG through `plotter.chart`. */
    renderChart?: (vegaLiteJson: string) => ChartRenderResponse;
    /** Derive incremental passes for one semantic runtime action. */
    runtimePassesForAction?: (action: string, target?: string) => RuntimePassResponse;
    /** Derive incremental passes for one or more invalidation scopes. */
    runtimePassesForScopes?: (scopes: string[]) => RuntimePassResponse;
    /** Lay out Markdown and return the PDF plus the run widths layout reserved. */
    pdfLayoutProbe?: (label: string, bytes: Uint8Array) => PdfLayoutProbeResponse;
    installLoggerLogFile?: (path: string) => void;
    installConsoleLogFile?: (path: string) => void;
};

export type PanduckWasmOptions = {
    /** Browser URL for `panduck_wasm_bg.wasm` when not using the default package layout. */
    url?: string | URL;
};
