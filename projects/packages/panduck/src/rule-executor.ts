/**
 * RuleBuilder plan executor (TS-side, linkedom-based).
 *
 * Flow:
 *   1. Parse input HTML into a linkedom document.
 *   2. Walk RulePlan steps in declaration order.
 *      - `input.select`: narrow current scope with CSS querySelectorAll.
 *      - `input.require`: assert a selector matches within the current scope.
 *      - `transform.remove`: delete matching nodes from the working document.
 *      - `transform.unwrap`: replace each matching node with its children.
 *      - `output.replace`: call the registered handler (DOM node → Markdown)
 *        and embed the output as an HTML-comment sentinel, then remove the
 *        original node from the DOM so Rust sees only the sentinel.
 *      - `output.require-no-loss`: flag for post-Rust check (not evaluated
 *        here — the executor just records it).
 *   3. Serialize the post-plan DOM back to HTML for Rust conversion.
 *   4. Post-process the Rust Markdown: find every sentinel comment and
 *      replace it with the handler's real Markdown output. If a sentinel is
 *      missing from the Rust output → a `panduck.executor.sentinel-lost`
 *      error diagnostic, because that means the Markdown transformer ate a
 *      piece of content the user explicitly asked to preserve.
 */

import { parseHTML } from "linkedom";
import type { RulePlan, InputOp, TransformOp, OutputOp, ReplaceHandlerFn, MarkdownHelper } from "./rule-builder.js";
import type { DiagnosticRecord } from "./converter.js";

export const SENTINEL_PREFIX = "<!-- panduck-replace:";
export const SENTINEL_SUFFIX = "-->";

/** Single sentinel record — keyed by the N in `<!-- panduck-replace:N -->`. */
type SentinelRecord = {
    /** Replace step index within its parent RuleStep. */
    replaceIndex: number;
    /** The handler's Markdown output (what we'll substitute in post-process). */
    handlerMarkdown: string;
    /** The selector that matched. */
    selector: string;
    /** The original DOM node (kept for potential loss diagnostics). */
    nodeTag?: string;
};

export type ExecutePlanResult = {
    /** HTML ready for Rust Markdown conversion. */
    html: string;
    /** All diagnostics collected during plan execution (NOT loss checks). */
    diagnostics: DiagnosticRecord[];
    /** Sentinel records for post-Rust Markdown substitution. */
    sentinels: Map<number, SentinelRecord>;
    /** Did any step declare require-no-loss? */
    requireNoLoss: boolean;
};

/** Execute a RulePlan against input HTML. */
export function executePlan(
    html: string,
    plan: RulePlan,
    handlers: Record<string, ReplaceHandlerFn>,
    urlHint?: string,
): ExecutePlanResult {
    const diagnostics: DiagnosticRecord[] = [];
    let requireNoLoss = false;

    // Parse into DOM. linkedom parseHTML returns { document, window }.
    const { document } = parseHTML(html);
    // Current scope starts at documentElement (<html>...</html>).
    let currentScope: Element[] = [document.documentElement];

    // Sentinel counter — each replace op gets a unique N.
    let sentinelCounter = 0;
    const sentinels = new Map<number, SentinelRecord>();

    for (let stepIdx = 0; stepIdx < plan.steps.length; stepIdx++) {
        const step = plan.steps[stepIdx];
        const { domain, ops } = step;

        if (ops.length === 0) continue;

        switch (domain) {
            case "input": {
                currentScope = executeInputStep(
                    stepIdx, ops as InputOp[], currentScope, document, diagnostics, urlHint,
                );
                break;
            }
            case "transform": {
                currentScope = executeTransformStep(
                    stepIdx, ops as TransformOp[], currentScope, document, diagnostics, urlHint,
                );
                break;
            }
            case "output": {
                const result = executeOutputStep(
                    stepIdx, ops as OutputOp[], currentScope, handlers, document, sentinels, sentinelCounter, diagnostics, urlHint,
                );
                sentinelCounter = result.nextCounter;
                if (result.requireNoLoss) requireNoLoss = true;
                // output ops do not change the selection scope — only input/transform do.
                break;
            }
        }
    }

    // Serialize the DOM back to HTML. We read documentElement.outerHTML —
    // linkedom does not expose a standalone serialize() function in its TS
    // surface, but every Element has outerHTML. The full document fragment is
    // what Rust's HTML reader expects (it handles both fragments and docs).
    const outHtml = document.documentElement?.outerHTML ?? "";

    return { html: outHtml, diagnostics, sentinels, requireNoLoss };
}

// ---------------------------------------------------------------------------
// Input scope — selection and assertions.
// ---------------------------------------------------------------------------

function executeInputStep(
    stepIdx: number,
    ops: InputOp[],
    scope: Element[],
    document: Document,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): Element[] {
    let currentScope = scope;
    for (let opIdx = 0; opIdx < ops.length; opIdx++) {
        const op = ops[opIdx];
        switch (op.op) {
            case "select": {
                const matches = querySelectorAllWithin(currentScope, op.selector);
                if (matches.length === 0) {
                    diagnostics.push({
                        code: "panduck.executor.select-empty",
                        fallback: `Input select found nothing for "${op.selector}" at step ${stepIdx}.${opIdx}`,
                        args: { step: stepIdx, op: opIdx, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "error",
                        stage: "structure",
                        url: urlHint,
                    });
                }
                currentScope = matches;
                break;
            }
            case "require": {
                const matches = querySelectorAllWithin(currentScope, op.selector);
                if (matches.length === 0) {
                    diagnostics.push({
                        code: "panduck.executor.required-missing",
                        fallback: `Required structure "${op.key}" (selector "${op.selector}") not found at step ${stepIdx}.${opIdx}`,
                        args: { step: stepIdx, op: opIdx, key: op.key, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "error",
                        stage: "structure",
                        url: urlHint,
                    });
                } else {
                    diagnostics.push({
                        code: "panduck.executor.required-found",
                        fallback: `Required structure "${op.key}" present (${matches.length} match)`,
                        args: { step: stepIdx, op: opIdx, key: op.key, selector: op.selector, count: matches.length },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "info",
                        stage: "structure",
                        url: urlHint,
                    });
                }
                // require does NOT change scope — we only assert.
                break;
            }
        }
    }
    return currentScope;
}

// ---------------------------------------------------------------------------
// Transform scope — structural mutations that are visible to later steps.
// ---------------------------------------------------------------------------

function executeTransformStep(
    stepIdx: number,
    ops: TransformOp[],
    scope: Element[],
    document: Document,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): Element[] {
    let currentScope = scope;
    for (let opIdx = 0; opIdx < ops.length; opIdx++) {
        const op = ops[opIdx];
        switch (op.op) {
            case "remove": {
                const matches = querySelectorAllWithin(currentScope, op.selector);
                for (const node of matches) {
                    node.remove();
                }
                diagnostics.push({
                    code: "panduck.executor.remove-done",
                    fallback: `Removed ${matches.length} node(s) matching "${op.selector}"`,
                    args: { step: stepIdx, op: opIdx, selector: op.selector, count: matches.length },
                    origin: { namespace: "panduck", component: "executor" },
                    severity: "info",
                    stage: "transform",
                    url: urlHint,
                });
                // Remove empty elements from current scope — they're gone from
                // the document now, queries against them in later steps would
                // be misleading.
                currentScope = currentScope.filter((el) => el.isConnected);
                break;
            }
            case "unwrap": {
                const matches = querySelectorAllWithin(currentScope, op.selector);
                for (const node of matches) {
                    const parent = node.parentNode;
                    if (!parent) continue;
                    // Move all children before the node, then remove it.
                    while (node.firstChild) {
                        parent.insertBefore(node.firstChild, node);
                    }
                    node.remove();
                }
                diagnostics.push({
                    code: "panduck.executor.unwrap-done",
                    fallback: `Unwrapped ${matches.length} container(s) matching "${op.selector}"`,
                    args: { step: stepIdx, op: opIdx, selector: op.selector, count: matches.length },
                    origin: { namespace: "panduck", component: "executor" },
                    severity: "info",
                    stage: "transform",
                    url: urlHint,
                });
                currentScope = currentScope.filter((el) => el.isConnected);
                break;
            }
        }
    }
    return currentScope;
}

// ---------------------------------------------------------------------------
// Output scope — handler execution + sentinel embedding + loss flags.
// ---------------------------------------------------------------------------

function executeOutputStep(
    stepIdx: number,
    ops: OutputOp[],
    scope: Element[],
    handlers: Record<string, ReplaceHandlerFn>,
    document: Document,
    sentinels: Map<number, SentinelRecord>,
    startCounter: number,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): { nextCounter: number; requireNoLoss: boolean } {
    let counter = startCounter;
    let requireNoLoss = false;

    for (let opIdx = 0; opIdx < ops.length; opIdx++) {
        const op = ops[opIdx];
        switch (op.op) {
            case "replace": {
                const handler = handlers[op.handlerKey];
                if (!handler) {
                    diagnostics.push({
                        code: "panduck.executor.replace-missing-handler",
                        fallback: `No handler registered for key "${op.handlerKey}" — replace op skipped`,
                        args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "error",
                        stage: "rule",
                        url: urlHint,
                    });
                    continue;
                }
                const matches = querySelectorAllWithin(scope, op.selector);
                for (const node of matches) {
                    const N = counter++;
                    // Build a minimal MarkdownHelper surface for the handler.
                    const mdHelper: MarkdownHelper = {
                        children: (n: unknown) => nodeChildrenAsText(n),
                        quote: (text: string) =>
                            text.split("\n").map((line) => `> ${line}`).join("\n"),
                        emphasis: (text: string) => `*${text}*`,
                        strong: (text: string) => `**${text}**`,
                    };
                    let handlerMarkdown = "";
                    try {
                        handlerMarkdown = String(handler(node, mdHelper) ?? "");
                    } catch (err) {
                        diagnostics.push({
                            code: "panduck.executor.replace-handler-threw",
                            fallback: `Handler "${op.handlerKey}" threw for selector "${op.selector}"`,
                            args: {
                                step: stepIdx, op: opIdx,
                                handlerKey: op.handlerKey, selector: op.selector,
                                error: err instanceof Error ? err.message : String(err),
                            },
                            origin: { namespace: "panduck", component: "executor" },
                            severity: "error",
                            stage: "rule",
                            url: urlHint,
                        });
                        // Still embed a sentinel so post-process finds something —
                        // we mark it as empty and the loss diagnostic handles it.
                    }
                    sentinels.set(N, {
                        replaceIndex: opIdx,
                        handlerMarkdown,
                        selector: op.selector,
                        nodeTag: (node as Element)?.tagName,
                    });
                    // Replace the node with an HTML comment sentinel. Rust's
                    // Markdown lowering preserves HTML comments verbatim, so
                    // this survives to the TS post-process stage where we
                    // extract and substitute the handler output.
                    const comment = document.createComment(
                        `${SENTINEL_PREFIX}${N}\n${handlerMarkdown}\n${SENTINEL_SUFFIX.replace("<!--", "").replace("-->", "").trim()}`,
                    );
                    node.replaceWith(comment);
                }
                diagnostics.push({
                    code: "panduck.executor.replace-done",
                    fallback: `Handler "${op.handlerKey}" replaced ${matches.length} node(s) for "${op.selector}"`,
                    args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector, count: matches.length },
                    origin: { namespace: "panduck", component: "executor" },
                    severity: "info",
                    stage: "rule",
                    url: urlHint,
                });
                break;
            }
            case "require-no-loss": {
                requireNoLoss = true;
                diagnostics.push({
                    code: "panduck.executor.require-no-loss-set",
                    fallback: "Markdown conversion must leave no known structural loss",
                    args: { step: stepIdx, op: opIdx },
                    origin: { namespace: "panduck", component: "executor" },
                    severity: "info",
                    stage: "output",
                    url: urlHint,
                });
                break;
            }
        }
    }

    return { nextCounter: counter, requireNoLoss };
}

// ---------------------------------------------------------------------------
// Post-process: substitute sentinels in the Rust Markdown output.
// ---------------------------------------------------------------------------

/**
 * Walk the Rust Markdown output looking for every `<!-- panduck-replace:N ... -->`
 * sentinel comment. Replace it with the handler's real Markdown output we
 * stored during DOM execution. Missing sentinels → error diagnostics (the
 * Rust transformer ate content the user asked to preserve).
 */
export function postProcessMarkdown(
    markdown: string,
    sentinels: Map<number, SentinelRecord>,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): string {
    let result = markdown;
    const seen = new Set<number>();

    // Match every sentinel comment regardless of internal Markdown content.
    const sentinelRe = /<!--\s*panduck-replace:(\d+)[\s\S]*?-->/g;
    result = result.replace(sentinelRe, (match, rawN) => {
        const N = parseInt(rawN, 10);
        seen.add(N);
        const rec = sentinels.get(N);
        if (!rec) {
            // Shouldn't happen — counter is monotonic and we register all of them.
            diagnostics.push({
                code: "panduck.executor.sentinel-unknown",
                fallback: `Sentinel N=${N} found in Markdown but no record exists`,
                args: { sentinel: N },
                origin: { namespace: "panduck", component: "executor" },
                severity: "error",
                stage: "output",
                url: urlHint,
            });
            return match;
        }
        // Return the handler's Markdown directly (without surrounding blank
        // lines — handler output already has the formatting it wants).
        return rec.handlerMarkdown;
    });

    // Any sentinel we registered but didn't find in the Markdown? That means
    // Rust's Markdown transformer dropped the HTML comment entirely — a real
    // loss because the handler's output is gone.
    for (const N of sentinels.keys()) {
        if (!seen.has(N)) {
            const rec = sentinels.get(N)!;
            diagnostics.push({
                code: "panduck.executor.sentinel-lost",
                fallback: `Sentinel N=${N} (selector "${rec.selector}") missing from Markdown — Rust transformer ate the handler output`,
                args: { sentinel: N, selector: rec.selector },
                origin: { namespace: "panduck", component: "executor" },
                severity: "error",
                stage: "output",
                url: urlHint,
            });
        }
    }

    return result;
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/**
 * Run querySelectorAll but only within the union of the given scope elements.
 * We can't use element.querySelectorAll across multiple roots, so we run
 * each root independently and deduplicate.
 */
function querySelectorAllWithin(scope: Element[], selector: string): Element[] {
    const seen = new Set<Element>();
    const out: Element[] = [];
    for (const root of scope) {
        try {
            const nodes = root.querySelectorAll(selector);
            nodes.forEach((n) => {
                const el = n as unknown as Element;
                if (!seen.has(el)) {
                    seen.add(el);
                    out.push(el);
                }
            });
        } catch {
            // Invalid CSS selector — surface in diagnostics from the caller,
            // but don't crash the executor.
        }
    }
    return out;
}

/** Best-effort fallback for MarkdownHelper.children — returns plain text. */
function nodeChildrenAsText(node: unknown): string {
    if (!node) return "";
    const el = node as unknown as { textContent?: string; innerText?: string };
    if (typeof el.textContent === "string") return el.textContent.trim();
    return "";
}
