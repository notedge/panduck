import { platform, arch } from "node:os";
import { resolve } from "node:path";

import {
    loadChartCases,
    loadDiagramCases,
    loadFormatManifest,
    loadIrCases,
    loadRuntimePasses,
} from "@notedge/panduck-fixtures";
import {
    bindingLocation,
    capabilities,
    runCase,
} from "./adapter.js";
import { aggregateCategory, evaluateExpectation, evaluateSnapshot } from "./assertions.js";
import { addAssertion } from "./accept.js";
import {
    expectationFile,
    loadConversionCases,
    loadExpectation,
    loadSnapshotCases,
    snapshotSource,
    snapshotValue,
    resultsRoot,
} from "./corpus.js";
import type {
    AssertionResult,
    CaseDefinition,
    CaseObservation,
    CaseResult,
    GapRecord,
    RunReport,
    SnapshotCaseDefinition,
} from "./types.js";
import { runIdFromDate, sha256 } from "./util.js";
import { writeBaseline, writeRun } from "./archive.js";

type Command = "run" | "list" | "verify" | "baseline" | "accept" | "help";
type Suite = "all" | "convert" | "notedown";

const USAGE = [
    "panduck-harness <command> [--suite all|convert|notedown] [--name <baseline>]",
    "",
    "  run        Execute the runnable suites and archive observations under results/",
    "  verify     Validate the checked-in corpus without loading the binding",
    "  list       Print the corpus inventory",
    "  baseline   Copy results/latest.json into baselines/<name>.json",
    "  accept     Add one passing assertion to a case expectation and record provenance",
    "             --case <id> --add '<json>' --by <author> --why <reason> --semantic <changed>",
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
            if (value !== "all" && value !== "convert" && value !== "notedown") {
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
        id: "gap-ir-validation",
        repo: "panduck",
        capability: "document.ir graph validation",
        neededEntry: "a binding entry that returns validation issues for an authored graph",
        evidence: "expectations/ir/cases.json",
        affectedCases: ["ir-*"],
    },
    {
        id: "gap-chart-render",
        repo: "panduck",
        capability: "plotter.chart lowering and render",
        neededEntry: "a binding entry that lowers a Vega-Lite spec to SVG",
        evidence: "fixtures/chart/manifest.json",
        affectedCases: ["chart-*"],
    },
    {
        id: "gap-diagram-render",
        repo: "panduck",
        capability: "plotter.diagram DOT, Mermaid and PlantUML render",
        neededEntry: "a binding entry that lowers a diagram language to SVG",
        evidence: "expectations/plotter/diagram/cases.json",
        affectedCases: ["diagram-*"],
    },
    {
        id: "gap-runtime-passes",
        repo: "panduck",
        capability: "plotter.runtime pass derivation",
        neededEntry: "a binding entry that derives invalidation steps for an action",
        evidence: "expectations/plotter/runtime/passes.json",
        affectedCases: ["runtime-*"],
    },
    {
        id: "gap-formats-inspect",
        repo: "panduck",
        capability: "document.formats package inspection and import",
        neededEntry: "an entry that inspects a package part or imports a single format sample",
        evidence: "fixtures/formats/manifest.json",
        affectedCases: ["formats-*"],
    },
];

type CaseRun = { readonly observation: CaseObservation; readonly result: CaseResult };

function describeCase(definition: CaseDefinition, expectation: ReturnType<typeof loadExpectation>): CaseRun {
    let observation: CaseObservation;
    try {
        observation = runCase(definition);
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
    const source = snapshotSource(definition);
    const snapshot = snapshotValue(definition);
    const assertions = evaluateSnapshot(definition, source, snapshot);
    const observation: CaseObservation = {
        caseId: definition.id,
        kind: definition.kind,
        legs: [],
        durationMs: 0,
        capabilities: { version: "unavailable", formats: [], conversions: [] },
        details: {
            sourceSha256: sha256(source),
            snapshotCount: snapshot.count,
            comparison: definition.comparison,
        },
    };
    return {
        observation,
        result: {
            id: definition.id,
            title: definition.title,
            kind: definition.kind,
            status: "fail",
            category: aggregateCategory(assertions),
            assertions,
            durationMs: 0,
            oracle: definition.oracle,
            gap: "gap-lexer-snapshot",
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
            const run = describeCase(definition, expectation);
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

function commandAccept(options: Readonly<Record<string, string>>): number {
    const caseId = options.case;
    const by = options.by;
    const why = options.why;
    const semantic = options.semantic;
    if (caseId === undefined || by === undefined || why === undefined || semantic === undefined) {
        console.error("accept needs `--case`, `--by`, `--why` and `--semantic`");
        return 2;
    }
    try {
        const result = addAssertion({
            caseId,
            assertionJson: options.add,
            assertionFile: options["add-file"],
            author: by,
            reason: why,
            semantic,
        });
        console.log(`accepted ${result.caseId} · assertions ${result.assertionCount}`);
        console.log(`expectation ${result.expectationPath}`);
        return 0;
    } catch (error) {
        console.error(error instanceof Error ? error.message : String(error));
        return 1;
    }
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
        case "list":
            return commandList();
        case "accept":
            return commandAccept(parsed.options);
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
