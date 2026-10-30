import type { PanduckBindings } from "../types.js";

/** Install a native JSON lines log sink when bindings expose the hook. */
export function configureConsoleBackend(
    bindings: PanduckBindings | null,
    logFile?: string,
): void {
    if (!logFile) {
        return;
    }
    const install = bindings?.installLoggerLogFile ?? bindings?.installConsoleLogFile;
    if (!install) {
        return;
    }
    install(logFile);
}
