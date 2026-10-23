/**
 * RuleBuilder plan executor (TS-side, linkedom-based).
 *
 * @deprecated RuleBuilder 整体 deprecated — 用 {@link defineConversion} +
 *   {@link converter.ts} ConversionPlan path 替代。本文件保留至 Rust 侧
 *   IR transform pass 落地后删除。
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
                    stepIdx, ops as OutputOp[], currentScope, document, handlers, diagnostics, urlHint,
                );
                if (result.requireNoLoss) requireNoLoss = true;
                if (result.domMutated) {
                    // Re-resolve scope roots after handler-driven DOM replacement
                    // so subsequent (if any) input/transform steps see the new tree.
                    currentScope = currentScope.filter((el) => el.isConnected);
                }
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
// Output scope — handler execution + loss flags.
//
// NOTE: The v0 design doc describes handler dispatch as deferred to the Rust
// Notedown/Oak lowering host. The Rust side does **not** yet accept a handler
// registry, so we bridge here in TS: find matches, call the user-supplied
// ReplaceHandlerFn, and replace each matched node with a text span carrying
// the handler-produced markdown. Rust HTML→Markdown lowering then naturally
// promotes that markdown text into the final output.
// ---------------------------------------------------------------------------

type OutputStepResult = {
    requireNoLoss: boolean;
    /** Whether any `replace` handler actually mutated the DOM (caller may want to refresh scope roots). */
    domMutated: boolean;
};

function executeOutputStep(
    stepIdx: number,
    ops: OutputOp[],
    scope: Element[],
    document: Document,
    handlers: Record<string, ReplaceHandlerFn>,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): OutputStepResult {
    let requireNoLoss = false;
    let domMutated = false;

    for (let opIdx = 0; opIdx < ops.length; opIdx++) {
        const op = ops[opIdx];
        switch (op.op) {
            case "replace": {
                const handler = handlers[op.handlerKey];
                if (!handler) {
                    diagnostics.push({
                        code: "panduck.executor.replace-handler-missing",
                        fallback: `Replace handler "${op.handlerKey}" not found in registry for selector "${op.selector}"`,
                        args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "error",
                        stage: "rule",
                        url: urlHint,
                    });
                    break;
                }

                const matches = querySelectorAllWithin(scope, op.selector, diagnostics, urlHint);
                if (matches.length === 0) {
                    diagnostics.push({
                        code: "panduck.executor.replace-no-match",
                        fallback: `Replace handler "${op.handlerKey}" found nothing for selector "${op.selector}"`,
                        args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector },
                        origin: { namespace: "panduck", component: "executor" },
                        severity: "warn",
                        stage: "rule",
                        url: urlHint,
                    });
                    break;
                }

                const mdHelper: MarkdownHelper = makeMarkdownHelper(document);

                // Walk backwards so removals don't shift indices.
                for (let i = matches.length - 1; i >= 0; i--) {
                    const node = matches[i];
                    let markdown: string;
                    try {
                        markdown = handler(node, mdHelper);
                    } catch (err) {
                        diagnostics.push({
                            code: "panduck.executor.replace-handler-threw",
                            fallback: `Handler "${op.handlerKey}" threw for selector "${op.selector}": ${err instanceof Error ? err.message : String(err)}`,
                            args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey, selector: op.selector },
                            origin: { namespace: "panduck", component: "executor" },
                            severity: "error",
                            stage: "rule",
                            url: urlHint,
                        });
                        continue;
                    }

                    if (typeof markdown !== "string") {
                        diagnostics.push({
                            code: "panduck.executor.replace-handler-non-string",
                            fallback: `Handler "${op.handlerKey}" did not return a string`,
                            args: { step: stepIdx, op: opIdx, handlerKey: op.handlerKey },
                            origin: { namespace: "panduck", component: "executor" },
                            severity: "error",
                            stage: "rule",
                            url: urlHint,
                        });
                        continue;
                    }

                    // Replace the original node with a span carrying the
                    // handler-produced markdown as plain text (not HTML).
                    // Rust lowering extracts text content, so this markdown
                    // flows straight through to the final Markdown output.
                    const wrapper = document.createElement("span");
                    wrapper.setAttribute("data-panduck-replaced", op.handlerKey);
                    wrapper.textContent = markdown;
                    node.replaceWith(wrapper);
                    domMutated = true;
                }

                diagnostics.push({
                    code: "panduck.executor.replace-executed",
                    fallback: `Ran replace handler "${op.handlerKey}" on ${matches.length} node(s) matching "${op.selector}"`,
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

    return { requireNoLoss, domMutated };
}

function makeMarkdownHelper(document: Document): MarkdownHelper {
    return {
        children(node: unknown): string {
            const el = node as Element | null;
            if (!el || !("innerText" in el)) return "";
            return el.textContent ?? "";
        },
        quote(text: string): string {
            return `> ${text.split(/\r?\n/).join("\n> ")}`;
        },
        emphasis(text: string): string {
            return `*${text}*`;
        },
        strong(text: string): string {
            return `**${text}**`;
        },
    };
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
