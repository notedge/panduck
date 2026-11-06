import { createRequire } from "node:module";

import type { PanduckBindings } from "../types.js";

type NativeNavEntry = {
    label: string;
    href: string;
    children: NativeNavEntry[];
};

type NativeBinding = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
    supportsConversion?: (from: string, to: string) => boolean;
    supportedConversions?: () => string[];
    probeContainerIndex?: (
        label: string,
        bytes: Uint8Array,
    ) => {
        success: boolean;
        format: string;
        outer: string;
        inner: string;
        parts: string[];
        diagnosticsJson: string;
    };
    probeContainerDecode?: (
        label: string,
        bytes: Uint8Array,
        partPath?: string,
    ) => {
        success: boolean;
        format: string;
        outer: string;
        inner: string;
        parts: Array<{
            path: string;
            compressionMethod: number;
            compressedSize: number;
            uncompressedSize: number;
            decodedSize: number;
        }>;
        diagnosticsJson: string;
    };
    probeNavigationToc?: (
        label: string,
        bytes: Uint8Array,
    ) => {
        success: boolean;
        entries: NativeNavEntry[];
        diagnosticsJson: string;
    };
    importDocument?: (
        from: string,
        label: string,
        bytes: Uint8Array,
        budgetsJson?: string,
        assets?: Record<string, Uint8Array>,
    ) => {
        success: boolean;
        graphJson: string | null;
        coverageJson: string;
        lossesJson: string;
        diagnosticsJson: string;
    };
    importOfficeDocument?: (
        from: string,
        label: string,
        bytes: Uint8Array,
    ) => {
        success: boolean;
        kind: string;
        sheets: string[];
        slides: number;
        graphJson: string | null;
        coverageJson: string;
        lossesJson: string;
        diagnosticsJson: string;
    };
    transformDocument?: (
        graphJson: string,
        transformsJson?: string,
    ) => {
        success: boolean;
        graphJson: string;
        passesJson: string;
        diagnosticsJson: string;
    };
    exportDocument?: (
        to: string,
        graphJson: string,
    ) => {
        success: boolean;
        text?: string | null;
        html?: string | null;
        binary?: Uint8Array | null;
        coverageJson: string;
        lossesJson: string;
        diagnosticsJson: string;
    };
    supportsMarkdownProject?: (from: string) => boolean;
    exportMarkdownProject?: (graphJson: string) => {
        success: boolean;
        indexMarkdown: string;
        chapters: Array<{ path: string; markdown: string }>;
        assets: Array<{ path: string; bytes: Uint8Array }>;
        unresolvedAssets: string[];
        coverageJson: string;
        lossesJson: string;
        diagnosticsJson: string;
    };
    installConsoleLogFile?: (path: string) => void;
    installLoggerLogFile?: (path: string) => void;
    lexDocument?: (
        language: string,
        source: string,
    ) => {
        success: boolean;
        tokens: Array<{
            kind: string;
            text: string;
            start: number;
            end: number;
            ignored: boolean;
        }>;
        diagnostics: string[];
    };
    renderDiagram?: (
        language: string,
        source: string,
    ) => {
        success: boolean;
        svg: string | null;
        diagnostics: Array<{
            code: string;
            severity: string;
            message: string;
            spanText: string;
        }>;
    };
    validateDocumentGraph?: (graphJson: string) => {
        valid: boolean;
        issues: Array<{
            code: string;
            path: string;
            detail: string;
        }>;
        error: string | null;
    };
    renderChart?: (vegaLiteJson: string) => {
        success: boolean;
        svg?: string;
        diagnostics: Array<{
            code: string;
            severity: string;
            message: string;
            spanText: string;
            path: string;
        }>;
    };
    runtimePassesForAction?: (action: string, target?: string) => {
        passes: string[];
        scopes: string[];
        error: string | null;
    };
    runtimePassesForScopes?: (scopes: string[]) => {
        passes: string[];
        scopes: string[];
        error: string | null;
    };
    pdfLayoutProbe?: (
        label: string,
        bytes: Uint8Array,
    ) => {
        success: boolean;
        pdf?: Uint8Array;
        runs: Array<{
            page: number;
            x: number;
            y: number;
            width: number;
            size: number;
            text: string;
            fontFamily: string;
            style: string;
        }>;
        pages: Array<{
            width: number;
            height: number;
        }>;
        diagnostics: string[];
        error: string | null;
    };
};

function copyNavEntry(entry: NativeNavEntry): NativeNavEntry {
    return { label: entry.label, href: entry.href, children: entry.children.map(copyNavEntry) };
}

const PLATFORM_PACKAGES: Record<string, string> = {
    "win32-x64": "@notedge/panduck-win32-x64",
    "linux-x64": "@notedge/panduck-linux-x64",
    "linux-arm64": "@notedge/panduck-linux-arm64",
    "darwin-x64": "@notedge/panduck-darwin-x64",
    "darwin-arm64": "@notedge/panduck-darwin-arm64",
};

/** Load the platform-specific Node-API binary from `@notedge/panduck-<platform>`. */
export function loadPanduckNode(): PanduckBindings {
    const key = `${process.platform}-${process.arch}`;
    const pkg = PLATFORM_PACKAGES[key];
    if (!pkg) {
        throw new Error(`Unsupported platform for Panduck native bindings: ${key}`);
    }
    const require = createRequire(import.meta.url);
    const binding = require(pkg).default as NativeBinding;
    return {
        panduckVersion: () => binding.panduckVersion(),
        supportedFormats: () => binding.supportedFormats(),
        isSupportedFormat: (name) => binding.isSupportedFormat(name),
        supportsConversion: binding.supportsConversion
            ? (from, to) => binding.supportsConversion!(from, to)
            : undefined,
        supportedConversions: binding.supportedConversions
            ? () => binding.supportedConversions!()
            : undefined,
        probeContainerIndex: binding.probeContainerIndex
            ? (label, bytes) => {
                  const response = binding.probeContainerIndex!(label, bytes);
                  return {
                      success: response.success,
                      format: response.format,
                      outer: response.outer,
                      inner: response.inner,
                      parts: response.parts,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        probeContainerDecode: binding.probeContainerDecode
            ? (label, bytes, partPath) => {
                  const response = binding.probeContainerDecode!(label, bytes, partPath);
                  return {
                      success: response.success,
                      format: response.format,
                      outer: response.outer,
                      inner: response.inner,
                      parts: response.parts.map((part) => ({
                          path: part.path,
                          compressionMethod: part.compressionMethod,
                          compressedSize: part.compressedSize,
                          uncompressedSize: part.uncompressedSize,
                          decodedSize: part.decodedSize,
                      })),
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        probeNavigationToc: binding.probeNavigationToc
            ? (label, bytes) => {
                  const response = binding.probeNavigationToc!(label, bytes);
                  return {
                      success: response.success,
                      entries: response.entries.map(copyNavEntry),
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        importDocument: binding.importDocument
            ? (from, label, bytes, budgetsJson, assets) => {
                  const response = binding.importDocument!(from, label, bytes, budgetsJson, assets);
                  return {
                      success: response.success,
                      graphJson: response.graphJson ?? null,
                      coverageJson: response.coverageJson,
                      lossesJson: response.lossesJson,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        importOfficeDocument: binding.importOfficeDocument
            ? (from, label, bytes) => {
                  const response = binding.importOfficeDocument!(from, label, bytes);
                  return {
                      success: response.success,
                      kind: response.kind,
                      sheets: response.sheets,
                      slides: response.slides,
                      graphJson: response.graphJson ?? null,
                      coverageJson: response.coverageJson,
                      lossesJson: response.lossesJson,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        transformDocument: binding.transformDocument
            ? (graphJson, transformsJson) => {
                  const response = binding.transformDocument!(graphJson, transformsJson);
                  return {
                      success: response.success,
                      graphJson: response.graphJson,
                      passesJson: response.passesJson,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        exportDocument: binding.exportDocument
            ? (to, graphJson) => {
                  const response = binding.exportDocument!(to, graphJson);
                  return {
                      success: response.success,
                      text: response.text ?? null,
                      html: response.html ?? null,
                      binary: response.binary ? new Uint8Array(response.binary) : null,
                      coverageJson: response.coverageJson,
                      lossesJson: response.lossesJson,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        supportsMarkdownProject: binding.supportsMarkdownProject
            ? (from) => binding.supportsMarkdownProject!(from)
            : undefined,
        exportMarkdownProject: binding.exportMarkdownProject
            ? (graphJson) => {
                  const response = binding.exportMarkdownProject!(graphJson);
                  return {
                      success: response.success,
                      indexMarkdown: response.indexMarkdown,
                      chapters: response.chapters.map((chapter) => ({
                          path: chapter.path,
                          markdown: chapter.markdown,
                      })),
                      assets: response.assets.map((asset) => ({
                          path: asset.path,
                          bytes: new Uint8Array(asset.bytes),
                      })),
                      unresolvedAssets: response.unresolvedAssets,
                      coverageJson: response.coverageJson,
                      lossesJson: response.lossesJson,
                      diagnosticsJson: response.diagnosticsJson,
                  };
              }
            : undefined,
        installLoggerLogFile: binding.installLoggerLogFile
            ? (path) => {
                  binding.installLoggerLogFile!(path);
              }
            : binding.installConsoleLogFile
              ? (path) => {
                    binding.installConsoleLogFile!(path);
                }
              : undefined,
        lexDocument: binding.lexDocument
            ? (language, source) => {
                  const response = binding.lexDocument!(language, source);
                  return {
                      success: response.success,
                      tokens: response.tokens.map((token) => ({
                          kind: token.kind,
                          text: token.text,
                          start: token.start,
                          end: token.end,
                          ignored: token.ignored,
                      })),
                      diagnostics: response.diagnostics,
                  };
              }
            : undefined,
        renderDiagram: binding.renderDiagram
            ? (language, source) => {
                  const response = binding.renderDiagram!(language, source);
                  return {
                      success: response.success,
                      svg: response.svg,
                      diagnostics: response.diagnostics.map((diagnostic) => ({
                          code: diagnostic.code,
                          severity: diagnostic.severity,
                          message: diagnostic.message,
                          spanText: diagnostic.spanText,
                      })),
                  };
              }
            : undefined,
        validateDocumentGraph: binding.validateDocumentGraph
            ? (graphJson) => {
                  const response = binding.validateDocumentGraph!(graphJson);
                  return {
                      valid: response.valid,
                      issues: response.issues.map((issue) => ({
                          code: issue.code,
                          path: issue.path,
                          detail: issue.detail,
                      })),
                      error: response.error ?? null,
                  };
              }
            : undefined,
        renderChart: binding.renderChart
            ? (vegaLiteJson) => {
                  const response = binding.renderChart!(vegaLiteJson);
                  return {
                      success: response.success,
                      svg: response.svg ?? null,
                      diagnostics: response.diagnostics.map((diagnostic) => ({
                          code: diagnostic.code,
                          severity: diagnostic.severity,
                          message: diagnostic.message,
                          spanText: diagnostic.spanText,
                          path: diagnostic.path,
                      })),
                  };
              }
            : undefined,
        runtimePassesForAction: binding.runtimePassesForAction
            ? (action, target) => {
                  const response = binding.runtimePassesForAction!(action, target);
                  return {
                      passes: response.passes,
                      scopes: response.scopes,
                      error: response.error ?? null,
                  };
              }
            : undefined,
        runtimePassesForScopes: binding.runtimePassesForScopes
            ? (scopes) => {
                  const response = binding.runtimePassesForScopes!(scopes);
                  return {
                      passes: response.passes,
                      scopes: response.scopes,
                      error: response.error ?? null,
                  };
              }
            : undefined,
        pdfLayoutProbe: binding.pdfLayoutProbe
            ? (label, bytes) => {
                  const response = binding.pdfLayoutProbe!(label, bytes);
                  return {
                      success: response.success,
                      pdf: response.pdf ? new Uint8Array(response.pdf) : undefined,
                      runs: response.runs.map((run) => ({
                          page: run.page,
                          x: run.x,
                          y: run.y,
                          width: run.width,
                          size: run.size,
                          text: run.text,
                          fontFamily: run.fontFamily,
                          style: run.style,
                      })),
                      pages: response.pages.map((page) => ({
                          width: page.width,
                          height: page.height,
                      })),
                      diagnostics: response.diagnostics,
                      error: response.error ?? null,
                  };
              }
            : undefined,
        installConsoleLogFile: binding.installConsoleLogFile
            ? (path) => {
                  binding.installConsoleLogFile!(path);
              }
            : undefined,
    };
}
