declare module '@notedge/panduck-unknown-wasm32/pkg/panduck_wasm.js' {
    export default function init(input?: { module_or_path?: string | URL }): Promise<unknown>;
    export function panduckVersion(): string;
    export function supportedFormats(): string[];
    export function isSupportedFormat(name: string): boolean;
}
