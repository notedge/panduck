import { createRequire } from "node:module";

import type { PanduckBindings } from "../types.js";

type NativeBinding = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
    supportsConversion?: (from: string, to: string) => boolean;
    supportedConversions?: () => string[];
    convertDocument?: (from: string, to: string, inputPath: string) => {
        exitCode: number;
        markdown?: string;
        binary?: Uint8Array;
        reportJson: string;
    };
    inspectIndex?: (inputPath: string) => {
        format: string;
        outer: string;
        inner: string;
        parts: string[];
        reportJson: string;
    };
    installConsoleLogFile?: (path: string) => void;
    inspectDecode?: (inputPath: string, partPath?: string) => {
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
        reportJson: string;
    };
};

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
        convertDocument: binding.convertDocument
            ? (from, to, inputPath) => {
                  const response = binding.convertDocument!(from, to, inputPath);
                  return {
                      exitCode: response.exitCode,
                      markdown: response.markdown,
                      binary: response.binary
                          ? new Uint8Array(response.binary)
                          : undefined,
                      reportJson: response.reportJson,
                  };
              }
            : undefined,
        inspectIndex: binding.inspectIndex
            ? (inputPath) => {
                  const response = binding.inspectIndex!(inputPath);
                  return {
                      format: response.format,
                      outer: response.outer,
                      inner: response.inner,
                      parts: response.parts,
                      reportJson: response.reportJson,
                  };
              }
            : undefined,
        inspectDecode: binding.inspectDecode
            ? (inputPath, partPath) => {
                  const response = binding.inspectDecode!(inputPath, partPath);
                  return {
                      format: response.format,
                      outer: response.outer,
                      inner: response.inner,
                      parts: response.parts,
                      reportJson: response.reportJson,
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
