/**
 * Frozen Panduck-owned diagnostic code namespace.
 *
 * Wire values are public: CLI exit-code mapping, report consumers and tests key
 * off these strings, so a value change is a breaking change. New Panduck codes
 * must be added here and referenced from the emitting site instead of written
 * as string literals.
 *
 * Codes owned by other crates are forwarded unchanged and intentionally absent
 * from this catalog: `panduck.adapter.*` and `panduck.semantic.loss` come from
 * `document.diagnostic`, `panduck.transform.pass-applied` comes from
 * `document.convert` pass reports, and `panduck.render.unsupported_language`
 * comes from the N-API render surface.
 */
export const PANDUCK_DIAGNOSTIC_CODES = {
    // Request stage: input admission and byte reads.
    remoteFetchDisabled: "panduck.input.remote-fetch-disabled",
    readNotReadable: "panduck.read.not-readable",
    readIoFailed: "panduck.read.io-failed",

    // Plan stage: capability and route gates.
    routeUnlisted: "panduck.route.unlisted",
    inspectNotImplemented: "panduck.inspect.not_implemented",
    cliBlocked: "panduck.cli.blocked",
    conversionPlanActive: "panduck.convert.conversion-plan-active",

    // Convert stage: primitive wiring, reports and panics.
    emptyInput: "panduck.convert.empty-input",
    bindingMissing: "panduck.convert.binding-missing",
    nativePanic: "panduck.convert.native-panic",
    rustLoss: "panduck.convert.rust-loss",
    reportCorrupt: "panduck.convert.report-corrupt",
    reportMissing: "panduck.convert.report-missing",
    notImplemented: "panduck.convert.not_implemented",
    deadlineExceeded: "panduck.convert.deadline-exceeded",
    transformNonExecutableHandler: "panduck.transform.non-executable-handler",

    // Output stage: publish and size guards.
    publishFailed: "panduck.convert.publish-failed",
    publishConflict: "panduck.publish.conflict",
    unsupportedPublishKind: "panduck.convert.unsupported-publish-kind",
    exportSizeExceeded: "panduck.export.size-exceeded",

    // Budget and partial-check outcomes.
    budgetExhausted: "panduck.budget.exhausted",
    checkPartial: "panduck.check.partial",
} as const;

/** Union of every Panduck-owned diagnostic code. */
export type PanduckDiagnosticCode = (typeof PANDUCK_DIAGNOSTIC_CODES)[keyof typeof PANDUCK_DIAGNOSTIC_CODES];

/** All frozen code values in declaration order. */
export const PANDUCK_DIAGNOSTIC_CODE_VALUES: readonly PanduckDiagnosticCode[] = Object.freeze(
    Object.values(PANDUCK_DIAGNOSTIC_CODES),
);
