/**
 * Panduck RuleBuilder — a declarative, domain-scoped plan builder for
 * document manipulation rules.
 *
 * Contract (per design doc):
 *   - `input`     — read / select / assert structure over the current scope.
 *   - `transform` — mutate the working document; subsequent `input` /
 *                   `transform` see the result.
 *   - `output`    — register target-format rendering rules and delivery
 *                   constraints. Does NOT close the plan; more `input` or
 *                   `transform` can follow.
 *
 * The three domains are **not** a state machine — they can appear in any
 * order and any number of times. Each domain callback produces one RuleStep
 * in the final RulePlan; order is preserved exactly as declared.
 *
 * RulePlan is a JSON-serializable description of the requested operations.
 * Custom node-handler callbacks (the `(node, md) => Markdown` passed to
 * `.replace()`) are **not** serialized — they keep a stable opaque `handlerKey`
 * in the plan so a future host that *does* run custom handlers (e.g. a JS
 * DOM evaluator or a Notedown plugin host) can match them up. Converter.toMarkdown
 * only consumes the plan description and preserves every requested operation
 * as a diagnostic when the Rust side cannot execute it yet.
 */

// ---------------------------------------------------------------------------
// Plan — the JSON-serializable deliverable
// ---------------------------------------------------------------------------

export type RuleDomain = "input" | "transform" | "output";

// Input ops — select scope or assert required structure.
export type InputOp =
    | { readonly op: "select"; readonly selector: string }
    | { readonly op: "require"; readonly key: string; readonly selector: string };

// Transform ops — mutate the working document; effects are visible to
// subsequent input / transform / output steps.
export type TransformOp =
    | { readonly op: "remove"; readonly selector: string }
    | { readonly op: "unwrap"; readonly selector: string };

// Output ops — register rendering rules and delivery constraints.
export type OutputOp =
    | {
          readonly op: "replace";
          readonly selector: string;
          /** Stable opaque key for the custom handler — host-matched, not serialized as code. */
          readonly handlerKey: string;
      }
    | { readonly op: "require-no-loss" };

export type RuleOp = InputOp | TransformOp | OutputOp;

/** One domain step — each `.input(...)` / `.transform(...)` / `.output(...)` creates exactly one. */
export type RuleStep = {
    readonly domain: RuleDomain;
    readonly ops: readonly RuleOp[];
};

/** Immutable, JSON-serializable rule plan — the `.build()` product. */
export type RulePlan = {
    readonly version: "panduck-rule-v0";
    readonly steps: readonly RuleStep[];
};

// ---------------------------------------------------------------------------
// Scope classes — TS type gates for what's legal in each domain
// ---------------------------------------------------------------------------

/** Read / select / assert structure. No mutation, no output rule registration. */
export class InputScope {
    private readonly _ops: InputOp[] = [];
    /** Internal — consumed by RuleBuilder after the callback returns. */
    get ops(): readonly InputOp[] {
        return this._ops;
    }

    /**
     * Continue narrowing the current scope with a CSS selector. The matched
     * region becomes the scope for *subsequent* input / transform / output
     * steps in the plan.
     */
    select(selector: string): this {
        this._ops.push({ op: "select", selector });
        return this;
    }

    /**
     * Assert that the given selector matches within the current scope. On
     * execution a missing match produces a structured diagnostic
     * (`panduck.rule.require-missing`) and — depending on the loss policy —
     * may fail the page. Does **not** change the scope.
     */
    require(key: string, selector: string): this {
        this._ops.push({ op: "require", key, selector });
        return this;
    }
}

/**
 * Mutate the working document. `.remove()` deletes matching nodes; `.unwrap()`
 * drops the wrapper but keeps children. Effects are immediately visible to
 * subsequent steps.
 */
export class TransformScope {
    private readonly _ops: TransformOp[] = [];
    get ops(): readonly TransformOp[] {
        return this._ops;
    }

    /** Delete every node matching `selector` from the working document. */
    remove(selector: string): this {
        this._ops.push({ op: "remove", selector });
        return this;
    }

    /** Strip the wrapper element, keep its children in place. */
    unwrap(selector: string): this {
        this._ops.push({ op: "unwrap", selector });
        return this;
    }
}

/**
 * Register target-format output rules. `.replace()` binds a selector to a
 * custom Markdown renderer callback; the callback itself is **not** part of
 * the serialized plan — only a stable `handlerKey` is stored.
 *
 * `.requireNoLoss()` declares that the final output for the current scope
 * must not contain known structural loss. It is placement-independent — the
 * executor evaluates it after all structural steps finish.
 */
export class OutputScope {
    private readonly _ops: OutputOp[] = [];
    get ops(): readonly OutputOp[] {
        return this._ops;
    }

    /**
     * Replace every node matching `selector` with the Markdown returned by
     * `handler(node, md)`. `md` is a lightweight helper surface (`.children(node)`,
     * `.quote(...)`, `.emphasis(...)`, ...) that the executor provides — the
     * scope only records the selector + a stable opaque key for the handler.
     */
    replace(selector: string, handler: (...args: unknown[]) => unknown): this {
        // handlerKey = handler.name ?? anonymous counter. Stable within one
        // RuleBuilder so execution can match them up; intentionally not
        // serializable as code.
        const key = handler.name || `anon-${anonCounter++}`;
        this._ops.push({ op: "replace", selector, handlerKey: key });
        return this;
    }

    /** Require that after all structural steps the final Markdown conversion leaves no known loss. */
    requireNoLoss(): this {
        this._ops.push({ op: "require-no-loss" });
        return this;
    }
}

let anonCounter = 0;

// ---------------------------------------------------------------------------
// RuleBuilder — the public entry
// ---------------------------------------------------------------------------

/**
 * Declare an ordered, domain-scoped RulePlan. Domains can be interleaved
 * freely — `input → transform → input → transform → output → input` is a
 * valid plan. Each `.domain(cb)` call appends one RuleStep; order is exact.
 */
export class RuleBuilder {
    private readonly _steps: RuleStep[] = [];

    /** Read / select / assert structure. */
    input(cb: (scope: InputScope) => void): this {
        const scope = new InputScope();
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "input", ops: [...scope.ops] });
        }
        return this;
    }

    /** Mutate the working document — effects visible to subsequent steps. */
    transform(cb: (scope: TransformScope) => void): this {
        const scope = new TransformScope();
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "transform", ops: [...scope.ops] });
        }
        return this;
    }

    /** Register target-format rendering rules and delivery constraints. */
    output(cb: (scope: OutputScope) => void): this {
        const scope = new OutputScope();
        cb(scope);
        if (scope.ops.length > 0) {
            this._steps.push({ domain: "output", ops: [...scope.ops] });
        }
        return this;
    }

    /** Produce the immutable, JSON-serializable RulePlan. */
    build(): RulePlan {
        return {
            version: "panduck-rule-v0",
            steps: [...this._steps],
        };
    }
}
