import { createRequire } from "node:module";

import type { PanduckBindings } from "../types.js";

type NativeBinding = {
    panduckVersion: () => string;
    supportedFormats: () => string[];
    isSupportedFormat: (name: string) => boolean;
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
    };
}
