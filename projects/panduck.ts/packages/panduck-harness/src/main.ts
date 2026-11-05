import { platform, arch } from "node:os";
import { resolve } from "node:path";

import {
    loadChartCases,
    loadDiagramCases,
    loadFormatManifest,
    loadIrCases,
    loadRuntimePasses,
    readChartSpec,
    readIrGraph,
    type ChartCase,
    type DiagramRejectionCase,
    type DiagramSpecCase,
    type IrCase,
    type RuntimePassCase,
} from "@notedge/panduck-fixtures";
import {
    bindingLocation,
    capabilities,
    lexSource,
    renderChartSource,
    renderDiagramSource,
    runCase,
    runtimePassesForActions,
    runtimePassesForScopes,
    validateGraphSource,
} from "./adapter.js";
import { aggregateCategory, evaluateExpectation, evaluateSnapshot } from "./assertions.js";
import { addAssertion } from "./accept.js";
import { analyzeSvgPaint } from "./paint.js";
import {
    expectationFile,
    loadConversionCases,
    loadExpectation,
    loadSnapshotCases,
    snapshotSourceText,
    snapshotValue,
    resultsRoot,
} from "./corpus.js";
import type {
    AssertionResult,
    CaseDefinition,
    CaseObservation,
    CaseResult,
    DiagramObservation,
    GapRecord,
    RunReport,
    SnapshotCaseDefinition,
} from "./types.js";
import { runIdFromDate, sha256 } from "./util.js";
import { writeBaseline, writeRun } from "./archive.js";
import { measureCoverage, printCoverage, writeCoverage } from "./coverage.js";
import { checkArtifactFreshness, formatFreshnessReport } from "./freshness.js";

type Command = "run" | "list" | "verify" | "baseline" | "accept" | "oracle" | "coverage" | "freshness" | "help";
type Suite = "all" | "convert" | "notedown" | "diagram" | "ir" | "chart" | "runtime";

const USAGE = [
    "panduck-harness <command> [--suite all|convert|notedown|diagram|ir|chart|runtime] [--name <baseline>]",
    "",
    "  run        Execute the runnable suites and archive observations under results/",
    "  verify     Validate the checked-in corpus without loading the binding",
    "  freshness  Reject a `.node` artifact older than the pinned `oak-*` / `document-*` revisions",
    "  coverage   Measure every spec feature and report the status distribution",
    "             --format markdown|html|tex|typst",
    "  list       Print the corpus inventory",
    "  baseline   Copy results/latest.json into baselines/<name>.json",
    "  accept     Add one passing assertion to a case expectation and record provenance",
    "             --case <id> --add '<json>' --why <reason> --semantic <changed> [--replace true]",
    "  oracle     Add one known red assertion as a reproduction oracle, records the observed status",
    "             --case <id> --add '<json>' --why <reason> --semantic <changed>",
].join("\n");

type ParsedArgs = {
    readonly command: Command;
    readonly suite: Suite;
    readonly name: string;
    readonly options: Readonly<Record<string, string>>;
};

function parseArgs(argv: readonly string[]): ParsedArgs {
    const command = (argv[0] ?? "help") as Command;
    let suite: Suite = "all";
    let name = "latest";
    const options: Record<string, string> = {};
    for (let index = 1; index < argv.length; index += 1) {
        const argument = argv[index];
        if (argument === "--suite") {
            const value = argv[index + 1];
            if (
                value !== "all" &&
                value !== "convert" &&
                value !== "notedown" &&
                value !== "diagram" &&
                value !== "ir" &&
                value !== "chart" &&
                value !== "runtime"
            ) {
                throw new Error(`unsupported suite: ${String(value)}`);
            }
            suite = value;
            index += 1;
            continue;
        }
        if (argument === "--name") {
            name = argv[index + 1] ?? "latest";
            index += 1;
            continue;
        }
        if (argument.startsWith("--")) {
            const value = argv[index + 1];
            if (value === undefined || value.startsWith("--")) {
                throw new Error(`${argument} needs a value`);
            }
            options[argument.slice(2)] = value;
            index += 1;
        }
    }
    return { command, suite, name, options };
}

function corpusIntegrity(check: string, ok: boolean, detail: string): AssertionResult {
    return ok
        ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
        : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "undetermined" };
}

function corpusLoad(check: string, load: () => number): AssertionResult {
    try {
        return corpusIntegrity(check, true, `${load()} rows loaded`);
    } catch (error) {
        return corpusIntegrity(check, false, error instanceof Error ? error.message : String(error));
    }
}

function corpusResults(): readonly CaseResult[] {
    const checks: readonly AssertionResult[] = [
        corpusLoad("formats-manifest", () => loadFormatManifest().cases.length),
        corpusLoad("chart-manifest", () => loadChartCases().length),
        corpusLoad("ir-expectations", () => loadIrCases().length),
        corpusLoad("plotter-diagram", () => loadDiagramCases().specs.length + loadDiagramCases().rejections.length),
        corpusLoad("plotter-runtime", () => loadRuntimePasses().length),
        corpusLoad("convert-manifest", () => loadConversionCases().length),
    ];
    return checks.map((result, index) => ({
        id: `corpus-${index}`,
        title: String(result.assertion.kind === "corpus-integrity" ? result.assertion.check : "corpus"),
        kind: "convert" as const,
        status: result.status === "pass" ? "pass" : "fail",
        category: result.status === "pass" ? undefined : "undetermined",
        assertions: [result],
        durationMs: 0,
    }));
}

const CORPUS_GAPS: readonly GapRecord[] = [
    {
        id: "gap-formats-inspect",
        repo: "panduck",
        capability: "document.formats package inspection and import",
        neededEntry: "an entry that inspects a package part or imports a single format sample",
        evidence: "fixtures/formats/manifest.json",
        affectedCases: ["formats-*"],
    },
    {
        id: "gap-markdown-footnotes",
        repo: "document.rs",
        capability: "markdown footnote placement in the HTML writer",
        neededEntry: "an entry that places footnote bodies and reference markers instead of dropping them and reporting two losses",
        evidence: "fixtures/convert/basic-markdown.md",
        affectedCases: ["basic-markdown-to-html"],
    },
];

type CaseRun = { readonly observation: CaseObservation; readonly result: CaseResult };

async function describeCase(definition: CaseDefinition, expectation: ReturnType<typeof loadExpectation>): Promise<CaseRun> {
    let observation: CaseObservation;
    try {
        observation = await runCase(definition);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        const detail = `binding could not run \`${definition.id}\`: ${message}`;
        const failedObservation: CaseObservation = {
            caseId: definition.id,
            kind: definition.kind,
            legs: [],
            durationMs: 0,
            capabilities: { version: "unavailable", formats: [], conversions: [] },
        };
        return {
            observation: failedObservation,
            result: {
                id: definition.id,
                title: definition.title,
                kind: definition.kind,
                status: "fail",
                category: "undetermined",
                assertions: [corpusIntegrity("binding-load", false, detail)],
                durationMs: 0,
            },
        };
    }
    const assertions = evaluateExpectation(observation, expectation);
    const category = aggregateCategory(assertions);
    return {
        observation,
        result: {
            id: definition.id,
            title: definition.title,
            kind: definition.kind,
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: observation.durationMs,
            oracle: null,
        },
    };
}

function describeSnapshot(definition: SnapshotCaseDefinition): CaseRun {
    const source = snapshotSourceText(definition);
    const snapshot = snapshotValue(definition);
    const lex = definition.language === null
        ? { success: false, tokens: [], diagnostics: [], error: undefined }
        : lexSource(definition.language, source);
    const assertions = evaluateSnapshot(definition, source, snapshot, lex);
    const category = aggregateCategory(assertions);
    const observation: CaseObservation = {
        caseId: definition.id,
        kind: definition.kind,
        legs: [],
        durationMs: 0,
        capabilities: { version: "unavailable", formats: [], conversions: [] },
        details: {
            sourceSha256: sha256(source),
            sourceLineEndings: "lf",
            snapshotCount: snapshot.count,
            lexerTokens: lex.tokens.length,
            lexerSuccess: lex.success,
            lexerDiagnostics: lex.diagnostics.length,
            comparison: definition.comparison,
            oracle: definition.oracle,
        },
    };
    return {
        observation,
        result: {
            id: definition.id,
            title: definition.title,
            kind: definition.kind,
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
            oracle: definition.oracle,
            gap: definition.language === null ? "gap-lexer-grammar" : lex.error?.startsWith("missing-entry:") ? "gap-lexer-entry" : undefined,
        },
    };
}

function environment(available: boolean): RunReport["environment"] {
    const base = { node: process.version, platform: platform(), arch: arch() };
    if (!available) {
        return { ...base, panduckVersion: "unavailable", formats: [], conversions: [] };
    }
    const snapshot = capabilities();
    const location = bindingLocation();
    return {
        ...base,
        panduckVersion: snapshot.version,
        formats: snapshot.formats,
        conversions: snapshot.conversions,
        bindingPath: location.path,
        bindingInsideRepo: location.insideRepo,
    };
}

function totals(cases: readonly CaseResult[]): RunReport["totals"] {
    const assertions = cases.flatMap((entry) => entry.assertions);
    return {
        cases: cases.length,
        passed: cases.filter((entry) => entry.status === "pass").length,
        failed: cases.filter((entry) => entry.status === "fail").length,
        assertions: assertions.length,
        assertionsPassed: assertions.filter((entry) => entry.status === "pass").length,
        assertionsFailed: assertions.filter((entry) => entry.status === "fail").length,
    };
}

function classifications(cases: readonly CaseResult[]): RunReport["classifications"] {
    return {
        "implementation-defect": cases.filter((entry) => entry.category === "implementation-defect").length,
        "stale-expectation": cases.filter((entry) => entry.category === "stale-expectation").length,
        undetermined: cases.filter((entry) => entry.category === "undetermined").length,
    };
}

function diagramRun(
    id: string,
    title: string,
    assertions: readonly AssertionResult[],
    details: Readonly<Record<string, unknown>>,
    gap?: string,
): CaseRun {
    const category = aggregateCategory(assertions);
    return {
        observation: {
            caseId: id,
            kind: "convert",
            legs: [],
            durationMs: 0,
            capabilities: { version: "unavailable", formats: [], conversions: [] },
            details,
        },
        result: {
            id,
            title,
            kind: "convert",
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
            gap,
        },
    };
}

function describeDiagramSpec(definition: DiagramSpecCase): CaseRun {
    const first = renderDiagramSource(definition.language, definition.source);
    if (first.error !== undefined) {
        return diagramRun(
            definition.id,
            `${definition.language} source renders`,
            [corpusIntegrity("diagram-entry", false, first.error)],
            { language: definition.language, error: first.error },
        );
    }
    const second = renderDiagramSource(definition.language, definition.source);
    const failures: AssertionResult[] = [];
    const assertion = (check: string, ok: boolean, detail: string): void => {
        failures.push(
            ok
                ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
                : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "implementation-defect" },
        );
    };
    assertion("diagram-lower", first.success, `success is ${first.success}`);
    assertion(
        "diagram-render",
        first.svg !== null && first.svg.includes("<svg"),
        first.svg === null ? "svg is missing" : `svg carries ${first.svg.length} bytes`,
    );
    const deterministic = first.svg !== null && first.svg === second.svg;
    assertion(
        "diagram-deterministic",
        deterministic,
        deterministic ? "two renders are byte identical" : "two renders differ",
    );
    const paint = first.svg === null ? null : analyzeSvgPaint(first.svg);
    const paintless = definition.paintless === true;
    assertion(
        "invisible-geometry",
        paintless || (paint?.painted ?? 0) > 0,
        paintless
            ? "the case declares itself paintless"
            : paint === null
              ? "no svg to measure"
              : `${paint.painted}/${paint.elements} geometry elements carry paint (${paint.attributePainted} attribute, ${paint.classPainted} class)`,
    );
    return diagramRun(
        definition.id,
        `${definition.language} source renders`,
        failures,
        {
            language: definition.language,
            svgBytes: first.svg?.length ?? 0,
            diagnostics: first.diagnostics.length,
            paint,
        },
    );
}

function describeDiagramRejection(definition: DiagramRejectionCase): CaseRun {
    const observation = renderDiagramSource(definition.language, definition.source);
    if (observation.error !== undefined) {
        return diagramRun(
            definition.id,
            `${definition.language} rejection is reported`,
            [corpusIntegrity("diagram-entry", false, observation.error)],
            { language: definition.language, error: observation.error },
        );
    }
    const diagnostic = observation.diagnostics[0];
    const assertion = (check: string, ok: boolean, detail: string): AssertionResult =>
        ok
            ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
            : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "implementation-defect" };
    const assertions = [
        assertion("diagram-rejected", !observation.success, `success is ${observation.success}`),
        assertion(
            "diagram-code",
            diagnostic?.code === definition.code,
            `code is ${diagnostic?.code ?? "absent"}, expected ${definition.code}`,
        ),
        assertion(
            "diagram-span",
            diagnostic?.spanText === definition.spanText,
            `spanText is ${JSON.stringify(diagnostic?.spanText ?? "")}, expected ${JSON.stringify(definition.spanText)}`,
        ),
    ];
    return diagramRun(
        definition.id,
        `${definition.language} rejection is reported`,
        assertions,
        { language: definition.language, code: diagnostic?.code ?? null, spanText: diagnostic?.spanText ?? null },
    );
}

function diagramRuns(): readonly CaseRun[] {
    const corpus = loadDiagramCases();
    return [
        ...corpus.specs.map(describeDiagramSpec),
        ...corpus.rejections.map(describeDiagramRejection),
    ];
}

function chartRun(
    id: string,
    title: string,
    assertions: readonly AssertionResult[],
    details: Readonly<Record<string, unknown>>,
): CaseRun {
    const category = aggregateCategory(assertions);
    return {
        observation: {
            caseId: id,
            kind: "convert",
            legs: [],
            durationMs: 0,
            capabilities: { version: "unavailable", formats: [], conversions: [] },
            details,
        },
        result: {
            id,
            title,
            kind: "convert",
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
        },
    };
}

function describeChartCase(definition: ChartCase): CaseRun {
    const source = readChartSpec(definition);
    const first = renderChartSource(source);
    if (first.error !== undefined) {
        return chartRun(
            definition.id,
            `${definition.language} ${definition.classification} sample`,
            [corpusIntegrity("chart-entry", false, first.error)],
            { language: definition.language, error: first.error },
        );
    }
    const assertion = (check: string, ok: boolean, detail: string): AssertionResult =>
        ok
            ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
            : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "implementation-defect" };
    if (definition.classification === "supported") {
        const second = renderChartSource(source);
        const deterministic = first.svg !== null && first.svg === second.svg;
        return chartRun(
            definition.id,
            `${definition.language} spec renders`,
            [
                assertion("chart-render", first.success, `success is ${first.success}`),
                assertion(
                    "chart-svg",
                    first.svg !== null && first.svg.includes("<svg"),
                    first.svg === null ? "svg is missing" : `svg carries ${first.svg.length} bytes`,
                ),
                assertion(
                    "chart-deterministic",
                    deterministic,
                    deterministic ? "two renders are byte identical" : "two renders differ",
                ),
            ],
            {
                language: definition.language,
                svgBytes: first.svg?.length ?? 0,
                diagnostics: first.diagnostics.length,
            },
        );
    }
    const expectedCode = String(definition.expect.code ?? "");
    const expectedSeverity = String(definition.expect.severity ?? "");
    const expectedSpan = String(definition.expect.spanText ?? "");
    const diagnostic = first.diagnostics[0];
    return chartRun(
        definition.id,
        `${definition.language} rejection is reported`,
        [
            assertion("chart-rejected", !first.success, `success is ${first.success}`),
            assertion(
                "chart-code",
                diagnostic?.code === expectedCode,
                `code is ${diagnostic?.code ?? "absent"}, expected ${expectedCode}`,
            ),
            assertion(
                "chart-severity",
                diagnostic?.severity === expectedSeverity,
                `severity is ${diagnostic?.severity ?? "absent"}, expected ${expectedSeverity}`,
            ),
            assertion(
                "chart-span",
                diagnostic?.spanText === expectedSpan,
                `spanText is ${JSON.stringify(diagnostic?.spanText ?? "")}, expected ${JSON.stringify(expectedSpan)}`,
            ),
        ],
        {
            language: definition.language,
            code: diagnostic?.code ?? null,
            severity: diagnostic?.severity ?? null,
            spanText: diagnostic?.spanText ?? null,
        },
    );
}

function chartRuns(): readonly CaseRun[] {
    return loadChartCases().map(describeChartCase);
}

function irRun(
    id: string,
    title: string,
    assertions: readonly AssertionResult[],
    details: Readonly<Record<string, unknown>>,
    gap?: string,
): CaseRun {
    const category = aggregateCategory(assertions);
    return {
        observation: {
            caseId: id,
            kind: "convert",
            legs: [],
            durationMs: 0,
            capabilities: { version: "unavailable", formats: [], conversions: [] },
            details,
        },
        result: {
            id,
            title,
            kind: "convert",
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
            gap,
        },
    };
}

function describeIrCase(definition: IrCase): CaseRun {
    const observation = validateGraphSource(readIrGraph(definition));
    if (observation.error !== undefined) {
        return irRun(
            definition.id,
            `${definition.area} graph ${definition.classification === "accept" ? "is valid" : "is rejected"}`,
            [corpusIntegrity("ir-entry", false, observation.error)],
            { area: definition.area, error: observation.error },
            "gap-ir-validation",
        );
    }
    const actual = observation.issues.map((issue) => issue.code).sort();
    const actualCodes = [...new Set(actual)];
    const expected = [...new Set(definition.issues)].sort();
    const sameCodes =
        actualCodes.length === expected.length && actualCodes.every((code, index) => code === expected[index]);
    const assertion = (check: string, ok: boolean, detail: string): AssertionResult =>
        ok
            ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
            : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "implementation-defect" };
    const expectedValidity = definition.classification === "accept";
    const assertions = [
        assertion(
            "ir-validity",
            observation.valid === expectedValidity,
            `valid is ${observation.valid}, expected ${expectedValidity}`,
        ),
        assertion(
            "ir-issue-codes",
            sameCodes,
            `issues are [${actual.join(", ")}], expected [${expected.join(", ")}]`,
        ),
    ];
    if (definition.issueCount !== undefined) {
        assertions.push(
            assertion(
                "ir-issue-count",
                observation.issues.length === definition.issueCount,
                `issue count is ${observation.issues.length}, expected ${definition.issueCount}`,
            ),
        );
    }
    return irRun(
        definition.id,
        `${definition.area} graph ${expectedValidity ? "is valid" : "is rejected"}`,
        assertions,
        {
            area: definition.area,
            contract: definition.contract,
            issueCodes: actual,
            issueCount: observation.issues.length,
        },
    );
}

function irRuns(): readonly CaseRun[] {
    return loadIrCases().map(describeIrCase);
}

function runtimeRun(
    id: string,
    title: string,
    assertions: readonly AssertionResult[],
    details: Readonly<Record<string, unknown>>,
    gap?: string,
): CaseRun {
    const category = aggregateCategory(assertions);
    return {
        observation: {
            caseId: id,
            kind: "convert",
            legs: [],
            durationMs: 0,
            capabilities: { version: "unavailable", formats: [], conversions: [] },
            details,
        },
        result: {
            id,
            title,
            kind: "convert",
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
            gap,
        },
    };
}

function describeRuntimeCase(definition: RuntimePassCase): CaseRun {
    const actions =
        definition.action
            ?.split(",")
            .map((entry) => entry.trim())
            .filter((entry) => entry.length > 0) ?? [];
    const observation =
        definition.trigger === "action"
            ? runtimePassesForActions(actions)
            : runtimePassesForScopes(definition.scope === undefined ? [] : [definition.scope]);
    const subject = definition.trigger === "action" ? actions.join(", ") : (definition.scope ?? definition.id);
    if (observation.error !== undefined) {
        return runtimeRun(
            definition.id,
            `${subject} derives no passes`,
            [corpusIntegrity("runtime-entry", false, observation.error)],
            { trigger: definition.trigger, subject, error: observation.error },
        );
    }
    const same =
        observation.passes.length === definition.passes.length &&
        observation.passes.every((pass, index) => pass === definition.passes[index]);
    const assertion = (check: string, ok: boolean, detail: string): AssertionResult =>
        ok
            ? { assertion: { kind: "corpus-integrity", check }, status: "pass", detail }
            : { assertion: { kind: "corpus-integrity", check }, status: "fail", detail, category: "implementation-defect" };
    return runtimeRun(
        definition.id,
        `${subject} derives [${definition.passes.join(", ")}]`,
        [
            assertion(
                "runtime-passes",
                same,
                `passes are [${observation.passes.join(", ")}], expected [${definition.passes.join(", ")}]`,
            ),
        ],
        {
            trigger: definition.trigger,
            subject,
            passes: observation.passes,
            scopes: observation.scopes,
        },
    );
}

function runtimeRuns(): readonly CaseRun[] {
    return loadRuntimePasses().map(describeRuntimeCase);
}

function report(suite: Suite, cases: readonly CaseResult[], bindingsAvailable: boolean): RunReport {
    const startedAt = new Date();
    return {
        schema: "panduck.harness.run",
        runId: runIdFromDate(startedAt),
        startedAt: startedAt.toISOString(),
        durationMs: cases.reduce((sum, entry) => sum + entry.durationMs, 0),
        suite,
        environment: environment(bindingsAvailable),
        totals: totals(cases),
        classifications: classifications(cases),
        cases,
        gaps: [...CORPUS_GAPS],
    };
}

function printReport(run: RunReport, archive: string): void {
    console.log(`suite ${run.suite} · cases ${run.totals.passed}/${run.totals.cases} pass`);
    console.log(
        `assertions ${run.totals.assertionsPassed}/${run.totals.assertions} · implementation-defect ${run.classifications["implementation-defect"]} · undetermined ${run.classifications.undetermined}`,
    );
    for (const entry of run.cases.filter((item) => item.status === "fail")) {
        const failure = entry.assertions.find((assertion) => assertion.status === "fail");
        console.log(`  fail ${entry.id} (${entry.category ?? "-"}): ${failure?.detail ?? "-"}`);
    }
    console.log(`archive ${archive}`);
}

async function commandRun(suite: Suite): Promise<number> {
    const freshness = checkArtifactFreshness();
    if (freshness.issues.length > 0) {
        console.error(formatFreshnessReport(freshness));
        return 1;
    }
    const cases: CaseResult[] = [];
    const observations: CaseObservation[] = [];
    let bindingsAvailable = true;
    try {
        capabilities();
    } catch {
        bindingsAvailable = false;
    }
    if (suite === "all" || suite === "convert") {
        for (const definition of loadConversionCases()) {
            const expectation = loadExpectation(definition.id, expectationFile(definition));
            const run = await describeCase(definition, expectation);
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    if (suite === "all" || suite === "notedown") {
        for (const definition of loadSnapshotCases()) {
            const run = describeSnapshot(definition);
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    if (suite === "all" || suite === "diagram") {
        for (const run of diagramRuns()) {
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    if (suite === "all" || suite === "ir") {
        for (const run of irRuns()) {
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    if (suite === "all" || suite === "chart") {
        for (const run of chartRuns()) {
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    if (suite === "all" || suite === "runtime") {
        for (const run of runtimeRuns()) {
            cases.push(run.result);
            observations.push(run.observation);
        }
    }
    const run = report(suite, cases, bindingsAvailable);
    const archive = writeRun(run, observations);
    printReport(run, archive);
    return run.totals.failed === 0 ? 0 : 1;
}

function commandVerify(): number {
    const cases = corpusResults();
    const failures = cases.filter((entry) => entry.status === "fail");
    for (const entry of cases) {
        console.log(`${entry.status === "pass" ? "ok  " : "fail"} ${entry.title}: ${entry.assertions[0].detail}`);
    }
    console.log(`corpus checks ${cases.length - failures.length}/${cases.length} pass`);
    return failures.length === 0 ? 0 : 1;
}

function commandList(): number {
    const formats = loadFormatManifest();
    const chart = loadChartCases();
    const ir = loadIrCases();
    const diagram = loadDiagramCases();
    const runtime = loadRuntimePasses();
    const convert = loadConversionCases();
    console.log(`formats ${formats.cases.length} · convert ${convert.length} · chart ${chart.length} · ir ${ir.length}`);
    console.log(`diagram specs ${diagram.specs.length} rejections ${diagram.rejections.length} · runtime ${runtime.length}`);
    for (const entry of formats.cases) {
        console.log(`  ${entry.id} <${entry.format}/${entry.classification}>`);
    }
    for (const entry of convert) {
        console.log(`  ${entry.id} <${entry.from}->${entry.to ?? entry.via}>`);
    }
    for (const entry of chart) {
        console.log(`  ${entry.id} <${entry.classification}>`);
    }
    return 0;
}

function commandAccept(options: Readonly<Record<string, string>>, redOracle: boolean): Promise<number> {
    const caseId = options.case;
    const why = options.why;
    const semantic = options.semantic;
    if (caseId === undefined || why === undefined || semantic === undefined) {
        console.error("accept needs `--case`, `--why` and `--semantic`");
        return Promise.resolve(2);
    }
    return addAssertion({
        caseId,
        assertionJson: options.add,
        assertionFile: options["add-file"],
        reason: why,
        semantic,
        replace: options.replace === "true",
        redOracle,
    })
        .then((accepted) => {
            console.log(`accepted ${accepted.caseId} · assertions ${accepted.assertionCount} · observed ${accepted.observedStatus}`);
            console.log(`expectation ${accepted.expectationPath}`);
            return 0;
        })
        .catch((error: unknown) => {
            console.error(error instanceof Error ? error.message : String(error));
            return 1;
        });
}

/** Entry point used by `bin/cli.mjs` and the examples. */
export async function main(argv: readonly string[]): Promise<number> {
    let parsed: ReturnType<typeof parseArgs>;
    try {
        parsed = parseArgs(argv);
    } catch (error) {
        console.error(error instanceof Error ? error.message : String(error));
        console.error(USAGE);
        return 2;
    }
    switch (parsed.command) {
        case "run":
            return commandRun(parsed.suite);
        case "verify":
            return commandVerify();
        case "coverage": {
            const report = measureCoverage(parsed.options.format ?? null);
            const archive = writeCoverage(report);
            printCoverage(report, archive);
            return report.totals["silent-loss"] > 0 ? 1 : 0;
        }
        case "list":
            return commandList();
        case "freshness": {
            const freshness = checkArtifactFreshness();
            console.log(formatFreshnessReport(freshness));
            return freshness.issues.length === 0 ? 0 : 1;
        }
        case "accept":
            return commandAccept(parsed.options, false);
        case "oracle":
            return commandAccept(parsed.options, true);
        case "baseline":
            console.log(writeBaseline(parsed.name));
            return 0;
        default:
            console.log(USAGE);
            console.log(`results root ${resolve(resultsRoot)}`);
            return 0;
    }
}

if (process.argv[1]?.endsWith("main.js")) {
    main(process.argv.slice(2)).then((code) => {
        process.exitCode = code;
    });
}
