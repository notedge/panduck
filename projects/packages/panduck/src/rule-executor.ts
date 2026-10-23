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
 *      - `output.replace`: **deferred registration only** — records selector +
 *        handlerKey in the plan step. Does NOT execute the handler, does NOT
 *        modify the DOM, does NOT embed sentinels. Handler execution happens
 *        on the Rust Notedown/Oak side via the registered handlers map.
 *      - `output.require-no-loss`: flag for post-conversion check.
 *   3. Serialize the **scope-root fragments** (not the whole documentElement).
 *   4. Pass to Rust conversion + handler dispatch.
 *
 * Previously this executor did linkedom DOM mutations → HTML re-serialize →
 * sentinel comment embed → Rust re-parse → sentinel regex substitution.
 * That entire sentinel bridge has been **deleted**. Custom handler execution
 * is deferred to the Rust Notedown/Oak lowering + plugin host.
 */

import { parseHTML } from "linkedom";
import type { RulePlan, InputOp, TransformOp, OutputOp, ReplaceHandlerFn, MarkdownHelper } from "./rule-builder.js";
import type { DiagnosticRecord } from "./converter.js";

export type ExecutePlanResult = {
    /** Scope-root HTML fragments ready for Rust conversion (not whole documentElement). */
    html: string;
    /** All diagnostics collected during plan execution. */
    diagnostics: DiagnosticRecord[];
    /** Handler registry keyed by handlerKey — Rust side consumes these to execute custom `.replace()` handlers. */
    handlers: Record<string, ReplaceHandlerFn>;
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
                    stepIdx, ops as OutputOp[], currentScope, document, diagnostics, urlHint,
                );
                if (result.requireNoLoss) requireNoLoss = true;
                // output ops do not change the selection scope — only input/transform do.
                break;
            }
        }
    }

    // Serialize **scope-root fragments**, not the whole documentElement.
    // Each root in currentScope produces its own outerHTML; multiple roots
    // are concatenated (Rust handles both fragments and full docs).
    let outHtml = "";
    for (const root of currentScope) {
        if (root) outHtml += root.outerHTML ?? "";
    }

    return { html: outHtml, diagnostics, handlers, requireNoLoss };
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
                const matches = querySelectorAllWithin(currentScope, op.selector, diagnostics, urlHint);
                if (matches.length === 0) {
                    diagnostics.push({
                        code: "panduck.executor.select-empty",
                        fallback: `Input select found nothing for "${op.selector}" at step ${stepIdx}.${opIdx}`,
                        args: { step: stepIdx, op: opIdx, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "warn",
                        stage: "structure",
                        url: urlHint,
                    });
                }
                currentScope = matches;
                break;
            }
            case "require": {
                const matches = querySelectorAllWithin(currentScope, op.selector, diagnostics, urlHint);
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
                const matches = querySelectorAllWithin(currentScope, op.selector, diagnostics, urlHint);
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
                currentScope = currentScope.filter((el) => el.isConnected);
                break;
            }
            case "unwrap": {
                const matches = querySelectorAllWithin(currentScope, op.selector, diagnostics, urlHint);
                for (const node of matches) {
                    const parent = node.parentNode;
                    if (!parent) continue;
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
// Output scope — deferred handler registration + loss flags.
//
// CRITICAL: We do NOT execute handlers here. We do NOT modify the DOM.
// We do NOT embed sentinels. Those responsibilities belong to the Rust
// Notedown/Oak lowering + handler dispatch host.
// ---------------------------------------------------------------------------

function executeOutputStep(
    stepIdx: number,
    ops: OutputOp[],
    scope: Element[],
    document: Document,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): { requireNoLoss: boolean } {
    let requireNoLoss = false;

    for (let opIdx = 0; opIdx < ops.length; opIdx++) {
        const op = ops[opIdx];
        switch (op.op) {
            case "replace": {
                // Only log that a replace rule was registered. Handler lookup
                // + execution happens on the Rust side with the handler map.
                diagnostics.push({
                    code: "panduck.executor.replace-registered",
                    fallback: `Registered replace handler "${op.handlerKey}" for selector "${op.selector}" (deferred to Rust lowering)`,
                    args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector },
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

    return { requireNoLoss };
}

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/**
 * Run querySelectorAll but only within the union of the given scope elements.
 * Invalid selectors produce an **error diagnostic** instead of silently returning empty.
 */
function querySelectorAllWithin(
    scope: Element[],
    selector: string,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): Element[] {
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
        } catch (err) {
            // Invalid CSS selector — surface as error diagnostic, do NOT crash.
            diagnostics.push({
                code: "panduck.executor.invalid-selector",
                fallback: `Invalid CSS selector "${selector}": ${err instanceof Error ? err.message : String(err)}`,
                args: { selector, error: err instanceof Error ? err.message : String(err) },
                origin: { namespace: "panduck", component: "executor" },
                severity: "error",
                stage: "structure",
                url: urlHint,
            });
        }
    }
    return out;
}
